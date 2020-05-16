use crate::models::{Block, Transaction};
use crate::system_contracts;
use dotenv::dotenv;
use serde_cbor::{from_slice, to_vec};
use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};
use vm::Env;

lazy_static! {
    static ref TRANSACTION_PROCESSING_TIME: Duration = Duration::from_secs(4);
}

lazy_static! {
    pub static ref PUBLIC_KEY: Vec<u8> = {
        dotenv().ok();
        let private_key = base64::decode(&env::var("PRIVATE_KEY").unwrap()).unwrap();
        private_key[32..64].to_vec()
    };
}
pub async fn apply_block(
    con: &mut vm::Client,
    mut vm_state: &mut vm::State, block: Block, transactions: Vec<Transaction>) {
    for transaction in transactions.into_iter() {
        run_transaction(&mut vm_state, &transaction.clone().into(), &block);
        remove_from_pending(&mut con.get_connection().unwrap(), &transaction.into()).await;
    }
}

pub async fn run_transactions(
    con: &mut vm::Connection,
    mut vm_state: &mut vm::State,
    block: &Block,
) -> Vec<Transaction> {
    let now = Instant::now();
    let mut completed_transactions: Vec<Transaction> = Default::default();
    while now.elapsed() < *TRANSACTION_PROCESSING_TIME {
        if let Some(transaction) = get_next_transaction(con).await {
            let completed_transaction = run_transaction(&mut vm_state, &transaction, &block);
            remove_from_processing(con, &transaction).await;
            completed_transactions.push(completed_transaction);
        } else {
            sleep(Duration::from_millis(1));
        }
    }
    completed_transactions
}

pub fn run_transaction(
    mut state: &mut vm::State,
    transaction: &vm::Transaction,
    block: &Block,
) -> Transaction {
    let env = env_from_block(block);
    let result = if system_contracts::is_system_contract(&transaction) {
        let result = system_contracts::run(transaction, &mut state, &env);
        result
    } else {
        let (result, gas_left) = transaction.run(&mut state, &env);
        let gas_used = transaction.gas_limit - gas_left.expect("no gas left") as u64;

        let env = vm::Env {
            caller: None,
            block_winner: PUBLIC_KEY.to_vec(),
            block_number: 0,
        };
        system_contracts::transfer(
            transaction,
            gas_used as u32,
            transaction.sender.clone(),
            env.block_winner.clone(),
        );
        result
    };
    Transaction::from(transaction.complete(result))
}

fn env_from_block(block: &Block) -> Env {
    vm::Env {
        block_number: block.number as u64,
        block_winner: block.winner.clone(),
        ..Default::default()
    }
}
async fn get_next_transaction(conn: &mut vm::Connection) -> Option<vm::Transaction> {
    let transaction_bytes: Vec<u8> = vm::redis::cmd("RPOPLPUSH")
        .arg("transactions::pending")
        .arg("transactions::processing")
        .query(conn)
        .unwrap();

    if transaction_bytes.len() == 0 {
        None
    } else {
        Some(from_slice::<vm::Transaction>(&transaction_bytes).unwrap())
    }
}

async fn remove_from_processing(redis: &mut vm::Connection, transaction: &vm::Transaction) {
    let transaction_bytes = to_vec(&transaction).unwrap();
    vm::redis::cmd("LREM")
        .arg("transactions::processing")
        .arg(0)
        .arg(transaction_bytes.as_slice())
        .query(redis)
        .unwrap()
}

async fn remove_from_pending(redis: &mut vm::Connection, transaction: &vm::Transaction) {
    let transaction_bytes = to_vec(&transaction).unwrap();
    vm::redis::cmd("LREM")
        .arg("transactions::pending")
        .arg(0)
        .arg(transaction_bytes.as_slice())
        .query::<u64>(redis)
        .unwrap();
}
