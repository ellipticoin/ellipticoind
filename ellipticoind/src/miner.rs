extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate vm;

use crate::api;
use crate::constants::SYSTEM_CONTRACT;
use crate::models::*;
use crate::schema::blocks::dsl::blocks;
use crate::system_contracts;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use hashfactor::hashfactor;
use serde_cbor::{from_slice, to_vec};
use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};

lazy_static! {
    static ref TRANSACTION_PROCESSING_TIME: Duration = std::time::Duration::from_secs(1);
}

lazy_static! {
    static ref PUBLIC_KEY: Vec<u8> = {
        dotenv().ok();
        let private_key = base64::decode(&env::var("PRIVATE_KEY").unwrap()).unwrap();
        private_key[32..64].to_vec()
    };
}
const HASHFACTOR_TARGET: u64 = 1;

pub fn get_best_block(db: &PgConnection) -> Option<Block> {
    blocks
        .order(crate::schema::blocks::dsl::number.desc())
        .first(db)
        .optional()
        .unwrap()
}

pub fn next_block(best_block: &Option<Block>) -> Block {
    best_block.as_ref().map_or(
        Block {
            number: 1,
            ..Default::default()
        },
        |Block { number, hash, .. }| Block {
            parent_hash: Some(hash.to_vec()),
            number: number + 1,
            ..Default::default()
        },
    )
}

pub async fn mine(db: PgConnection, mut api: &mut api::API, mut vm_state: &mut vm::State) {
    let mut best_block = get_best_block(&db);
    loop {
        let (next_block, transactions) = mine_next_block(&mut api, &mut vm_state, best_block).await;
        println!("Mined Block #{}", next_block.clone().number);
        diesel::dsl::insert_into(crate::schema::blocks::dsl::blocks)
            .values(&next_block)
            .execute(&db)
            .unwrap();
        diesel::dsl::insert_into(crate::schema::transactions::dsl::transactions)
            .values(&transactions)
            .execute(&db)
            .unwrap();
        api.broadcast_block(api::Block::from((&next_block, &transactions)))
            .await;
        best_block = Some(next_block);
    }
}

async fn mine_next_block(
    api: &mut api::API,
    vm_state: &mut vm::State,
    best_block: Option<Block>,
) -> (Block, Vec<Transaction>) {
    let mut block = next_block(&best_block);
    block.winner = PUBLIC_KEY.to_vec();
    let env = vm::Env {
        block_number: block.number as u64,
        block_winner: PUBLIC_KEY.to_vec(),
        ..Default::default()
    };
    let mut transactions = run_transactions(api, vm_state, &env).await;
    // let mut rng = rand::thread_rng();
    // let random = rng.gen_range(0, 5000);
    // std::thread::sleep(std::time::Duration::from_millis(random));
    std::thread::sleep(std::time::Duration::from_secs(3));
    let encoded_block = serde_cbor::to_vec(&UnminedBlock::from(&block)).unwrap();
    block.proof_of_work_value = hashfactor(encoded_block, HASHFACTOR_TARGET) as i64;
    block.set_hash();
    transactions.iter_mut().for_each(|transaction| {
        transaction.set_hash();
        transaction.block_hash = block.hash.clone();
    });
    (block, transactions)
}

async fn run_transactions(
    api: &mut api::API,
    mut vm_state: &mut vm::State,
    env: &vm::Env,
) -> Vec<Transaction> {
    let now = Instant::now();
    let mut completed_transactions: Vec<Transaction> = Default::default();
    let mut con = vm::redis::Client::get_async_connection(&api.redis)
        .await
        .unwrap();
    let mut block_winner_tx_count = 0;
    while now.elapsed() < *TRANSACTION_PROCESSING_TIME {
        if let Some(transaction) = get_next_transaction(&mut con).await {
            if transaction.sender == PUBLIC_KEY.to_vec() {
                block_winner_tx_count += 1;
            }
            let completed_transaction = run_transaction(&mut vm_state, &transaction, env);
            remove_from_processing(&mut con, &transaction).await;
            completed_transactions.push(completed_transaction);
        } else {
            sleep(Duration::from_millis(1));
        }
    }
    let db = api.db.get().unwrap();
    let sender_nonce = next_nonce(&db, PUBLIC_KEY.to_vec()) + block_winner_tx_count;
    let mint_transaction = vm::Transaction {
        contract_address: SYSTEM_CONTRACT.to_vec(),
        sender: PUBLIC_KEY.to_vec(),
        nonce: sender_nonce,
        function: "mint".to_string(),
        arguments: vec![],
        gas_limit: 10000000,
    };
    completed_transactions.push(run_transaction(&mut vm_state, &mint_transaction, env));
    completed_transactions
}

fn run_transaction(
    mut state: &mut vm::State,
    transaction: &vm::Transaction,
    env: &vm::Env,
) -> Transaction {
    let (_transaction_memory_changeset, _transaction_storage_changeset, result) =
        if system_contracts::is_system_contract(&transaction) {
            system_contracts::run(transaction, env)
        } else {
            let (memory_changeset, storage_changeset, (result, gas_left)) =
                transaction.run(env, &mut state);
            let gas_used = transaction.gas_limit - gas_left.expect("no gas left") as u64;

            let env = vm::Env {
                caller: None,
                block_winner: PUBLIC_KEY.to_vec(),
                block_number: 0,
            };
            let (gas_memory_changeset, _, _) = system_contracts::transfer(
                transaction,
                memory_changeset,
                gas_used as u32,
                transaction.sender.clone(),
                env.block_winner.clone(),
            );
            (gas_memory_changeset, storage_changeset, result)
        };
    Transaction::from(transaction.complete(result))
}

async fn get_next_transaction(conn: &mut vm::Connection) -> Option<vm::Transaction> {
    let transaction_bytes: Vec<u8> = vm::redis::cmd("RPOPLPUSH")
        .arg("transactions::pending")
        .arg("transactions::processing")
        .query_async(conn)
        .await
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
        .query_async(redis)
        .await
        .unwrap()
}
