use crate::api;
use crate::constants::TOKEN_CONTRACT;
use crate::models::{next_nonce, Block, Transaction};
use crate::system_contracts;
use dotenv::dotenv;
use serde_cbor::{from_slice, to_vec};
use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};
use vm::Commands;
use vm::Env;

lazy_static! {
    static ref TRANSACTION_PROCESSING_TIME: Duration = std::time::Duration::from_secs(1);
}

lazy_static! {
    pub static ref PUBLIC_KEY: Vec<u8> = {
        dotenv().ok();
        let private_key = base64::decode(&env::var("PRIVATE_KEY").unwrap()).unwrap();
        private_key[32..64].to_vec()
    };
}
pub fn apply_block(mut vm_state: &mut vm::State, block: Block, transactions: Vec<Transaction>) {
    let env = env_from_block(&block);
    transactions.into_iter().for_each(|transaction| {
        run_transaction(&mut vm_state, &transaction.into(), &env);
    });
    println!("Applied block #{}", &block.clone().number);
}

pub async fn run_transactions(
    api: &mut api::State,
    mut vm_state: &mut vm::State,
    block: &Block,
) -> Vec<Transaction> {
    let env = env_from_block(block);
    let now = Instant::now();
    let mut completed_transactions: Vec<Transaction> = Default::default();
    let mut con = vm::redis::Client::get_connection(&api.redis).unwrap();
    let mut block_winner_tx_count = 0;
    while now.elapsed() < *TRANSACTION_PROCESSING_TIME {
        if let Some(transaction) = get_next_transaction(&mut con).await {
            if transaction.sender == PUBLIC_KEY.to_vec() {
                block_winner_tx_count += 1;
            }
            let completed_transaction = run_transaction(&mut vm_state, &transaction, &env);
            remove_from_processing(&mut con, &transaction).await;
            completed_transactions.push(completed_transaction);
        } else {
            sleep(Duration::from_millis(1));
        }
    }
    let db = api.db.get().unwrap();
    let sender_nonce = next_nonce(&db, PUBLIC_KEY.to_vec()) + block_winner_tx_count;
    let mint_transaction = vm::Transaction {
        contract_address: TOKEN_CONTRACT.to_vec(),
        sender: PUBLIC_KEY.to_vec(),
        nonce: sender_nonce,
        function: "mint".to_string(),
        arguments: vec![],
        gas_limit: 10000000,
    };
    completed_transactions.push(run_transaction(&mut vm_state, &mint_transaction, &env));
    completed_transactions
}

fn run_transaction(
    mut state: &mut vm::State,
    transaction: &vm::Transaction,
    env: &vm::Env,
) -> Transaction {
    let result = if system_contracts::is_system_contract(&transaction) {
        let result = system_contracts::run(transaction, &mut state, env);
        result
    } else {
        let (result, gas_left) = transaction.run(&mut state, env);
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
    if conn.llen::<&str, i32>("transactions::pending").unwrap_or(0) > 0 {
        // println!("longer than 0!");
    }
    let transaction_bytes: Vec<u8> = vm::redis::cmd("RPOPLPUSH")
        .arg("transactions::pending")
        .arg("transactions::processing")
        .query(conn)
        .unwrap();

    if transaction_bytes.len() == 0 {
        None
    } else {
        println!("got one!");
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
