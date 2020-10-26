use crate::{
    client::post_transaction,
    config::my_public_key,
    constants::{MINERS, TRANSACTION_QUEUE},
    models::transaction::Transaction,
    transaction::TransactionRequest,
};
use async_std::{future::Future, prelude::FutureExt as asyncStdFutureExt, task::sleep};
use futures::future::FutureExt;
use serde_cbor::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub fn sha256(message: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(message);
    hasher.finalize().into()
}
/*
 * Until Rust has [specialization](https://github.com/rust-lang/rust/issues/31844) we need to
 * encode Vec<u8> values as Vec<Value>
*/
pub fn bytes_to_value(bytes: Vec<u8>) -> Value {
    bytes
        .into_iter()
        .map(|n| n.into())
        .collect::<Vec<Value>>()
        .into()
}

pub async fn run_transaction(transaction_request: TransactionRequest) -> Transaction {
    if MINERS.current().await.address == my_public_key() {
        let receiver = TRANSACTION_QUEUE.push(transaction_request).await;
        receiver.await.unwrap()
    } else {
        post_transaction(&MINERS.current().await.host, transaction_request).await
    }
}

#[cfg(test)]
pub fn generate_hash_onion(layers: usize, center: [u8; 32]) -> Vec<[u8; 32]> {
    let mut onion = vec![center];
    for _ in 1..(layers) {
        onion.push(sha256(onion.last().unwrap().to_vec()));
    }
    onion
}

pub async fn run_for<F>(duration: Duration, f: F)
where
    F: Future<Output = ()>,
{
    sleep(duration)
        .join(f)
        .map(|_| ())
        .race(sleep(duration))
        .await;
}
