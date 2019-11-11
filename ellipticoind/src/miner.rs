extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate vm;

use crate::api;
use crate::system_contracts;
use crate::tokio::future::FutureExt;
use serde_cbor::{from_slice, to_vec};
use std::time::Duration;
use vm::{CompletedTransaction, Env, Transaction};

const TRANSACTION_PROCESSING_TIME: u64 = 1000;

pub async fn mine(mut api: &mut api::API, mut vm_state: &mut vm::State) {
    loop {
        mine_next_block(&mut api, &mut vm_state).await;
    }
}

async fn mine_next_block(api: &mut api::API, vm_state: &mut vm::State) {
    let env = vm::Env {
        caller: None,
        block_winner: vec![],
        block_hash: vec![],
        block_number: 0,
    };
    run_transactions(api, vm_state, &env)
        .timeout(Duration::from_millis(TRANSACTION_PROCESSING_TIME))
        .await
        .unwrap_err();
}

async fn get_next_transaction(conn: &mut vm::Connection) -> Transaction {
    let transaction_bytes: Vec<u8> = vm::redis::cmd("BRPOPLPUSH")
        .arg("transactions::pending")
        .arg("transactions::processing")
        .arg::<u32>(0)
        .query_async(conn)
        .await
        .unwrap();
    from_slice::<Transaction>(&transaction_bytes).unwrap()
}

async fn run_transactions(api: &mut api::API, mut vm_state: &mut vm::State, env: &vm::Env) {
    let mut completed_transactions: Vec<CompletedTransaction> = Default::default();
    let mut con = vm::redis::Client::get_async_connection(&api.redis)
        .await
        .unwrap();
    loop {
        let transaction = get_next_transaction(&mut con).await;
        let completed_transaction = run_transaction(&mut vm_state, &transaction, env);
        remove_from_processing(&mut con, &transaction).await;
        completed_transactions.push(completed_transaction);
    }
}

fn run_transaction(
    mut state: &mut vm::State,
    transaction: &vm::Transaction,
    env: &vm::Env,
) -> CompletedTransaction {
    let (_transaction_memory_changeset, _transaction_storage_changeset, result) =
        if system_contracts::is_system_contract(&transaction) {
            system_contracts::run(transaction, env)
        } else {
            let (memory_changeset, storage_changeset, (result, gas_left)) =
                transaction.run(env, &mut state);
            let gas_used = transaction.gas_limit - gas_left.expect("no gas left") as u64;

            let env = vm::Env {
                caller: None,
                block_winner: vec![],
                block_hash: vec![],
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
    transaction.complete(result)
}

async fn remove_from_processing(redis: &mut vm::Connection, transaction: &Transaction) {
    let transaction_bytes = to_vec(&transaction).unwrap();
    vm::redis::cmd("LREM")
        .arg("transactions::processing")
        .arg(0)
        .arg(transaction_bytes.as_slice())
        .query_async(redis)
        .await
        .unwrap()
}
