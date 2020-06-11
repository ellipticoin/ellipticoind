use crate::models::{Block, Transaction};
use crate::system_contracts;
use async_std::task;

use crate::vm::Env;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use dotenv::dotenv;
use futures::{future::FutureExt, pin_mut, select};
use serde_cbor::{from_slice, to_vec};
use std::env;
use std::ops::DerefMut;
use std::time::Duration;

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
    mut redis: crate::vm::r2d2_redis::r2d2::PooledConnection<
        crate::vm::r2d2_redis::RedisConnectionManager,
    >,
    mut vm_state: &mut crate::vm::State,
    block: Block,
    transactions: Vec<Transaction>,
    db: PooledConnection<ConnectionManager<PgConnection>>,
) {
    for transaction in transactions.clone().into_iter() {
        run_transaction(&mut vm_state, &transaction.clone().into(), &block);
        remove_from_pending(&mut redis, &transaction.into()).await;
    }
    vm_state.commit();
    block.insert(&db, transactions);
}

pub async fn run_transactions(
    pool: crate::vm::r2d2_redis::r2d2::Pool<crate::vm::r2d2_redis::RedisConnectionManager>,
    mut vm_state: &mut crate::vm::State,
    block: &Block,
) -> Vec<Transaction> {
    let mut completed_transactions: Vec<Transaction> = Default::default();
    let timer = task::sleep(*TRANSACTION_PROCESSING_TIME).fuse();
    pin_mut!(timer);
    loop {
        let mut con = pool.get().unwrap();
        let get_next_transaction_fused = get_next_transaction(&mut con).fuse();
        pin_mut!(get_next_transaction_fused);
        select! {
            transaction = get_next_transaction_fused => {
                    let mut con = pool.get().unwrap();
                    let completed_transaction = run_transaction(&mut vm_state, &transaction, &block);
                    remove_from_processing(&mut con, &transaction).await;
                    completed_transactions.push(completed_transaction);
            },
            _ = timer => break,
        };
    }
    completed_transactions
}

pub fn run_transaction(
    mut state: &mut crate::vm::State,
    transaction: &crate::vm::Transaction,
    block: &Block,
) -> Transaction {
    let env = env_from_block(block);
    let result = if system_contracts::is_system_contract(&transaction) {
        let result = system_contracts::run(transaction, &mut state, &env);
        result
    } else {
        let (result, gas_left) = transaction.run(&mut state, &env);
        let gas_used = transaction.gas_limit - gas_left.expect("no gas left") as u64;

        let env = crate::vm::Env {
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
    crate::vm::Env {
        block_number: block.number as u64,
        block_winner: block.winner.clone(),
        ..Default::default()
    }
}
async fn get_next_transaction(
    redis: &mut crate::vm::r2d2_redis::r2d2::PooledConnection<
        crate::vm::r2d2_redis::RedisConnectionManager,
    >,
) -> crate::vm::Transaction {
    loop {
        let transaction_bytes: Vec<u8> = crate::vm::redis::cmd("RPOPLPUSH")
            .arg("transactions::pending")
            .arg("transactions::processing")
            .query(redis.deref_mut())
            .unwrap();
        if transaction_bytes.len() > 0 {
            return from_slice::<crate::vm::Transaction>(&transaction_bytes).unwrap();
        }
        task::sleep(Duration::from_millis(50)).await;
    }
}

async fn remove_from_processing(
    redis: &mut crate::vm::r2d2_redis::r2d2::PooledConnection<
        crate::vm::r2d2_redis::RedisConnectionManager,
    >,
    transaction: &crate::vm::Transaction,
) {
    let transaction_bytes = to_vec(&transaction).unwrap();
    crate::vm::redis::cmd("LREM")
        .arg("transactions::processing")
        .arg(0)
        .arg(transaction_bytes.as_slice())
        .query(redis.deref_mut())
        .unwrap()
}

async fn remove_from_pending(
    redis: &mut crate::vm::r2d2_redis::r2d2::PooledConnection<
        crate::vm::r2d2_redis::RedisConnectionManager,
    >,
    transaction: &crate::vm::Transaction,
) {
    let transaction_bytes = to_vec(&transaction).unwrap();
    crate::vm::redis::cmd("LREM")
        .arg("transactions::pending")
        .arg(0)
        .arg(transaction_bytes.as_slice())
        .query::<u64>(redis.deref_mut())
        .unwrap();
}