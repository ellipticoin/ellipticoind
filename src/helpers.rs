use crate::{
    api::views,
    config::{Bootnode, HOST},
    state::MINERS,
    system_contracts::ellipticoin::Miner,
    transaction::Transaction,
};
use rand::Rng;
use serde_cbor::Value;
use sha2::{Digest, Sha256};

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

#[cfg(test)]
pub fn generate_hash_onion(layers: usize, center: [u8; 32]) -> Vec<[u8; 32]> {
    let mut onion = vec![center];
    for _ in 1..(layers) {
        onion.push(sha256(onion.last().unwrap().to_vec()));
    }
    onion
}

pub fn random() -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0, u32::max_value() as u32)
}

pub async fn post_transaction(bootnode: &Bootnode, transaction: Transaction) {
    let uri = format!("http://{}/transactions", bootnode.host);
    surf::post(uri)
        .body_bytes(serde_cbor::to_vec(&transaction).unwrap())
        .await
        .unwrap();
}

pub async fn get_block(bootnode: &Bootnode, block_number: u32) -> Option<views::Block> {
    let url = format!("http://{}/blocks/{}", bootnode.host, block_number);
    let mut res = surf::get(url).await.unwrap();
    if res.status() == 200 {
        serde_cbor::from_slice::<views::Block>(&res.body_bytes().await.unwrap())
            .unwrap()
            .into()
    } else {
        None
    }
}

pub async fn current_miner() -> Miner {
    MINERS.lock().await.clone().first().unwrap().clone()
}

pub async fn peers() -> Vec<String> {
    MINERS
        .lock()
        .await
        .clone()
        .iter()
        .map(|miner| miner.host.clone())
        .filter(|host| host.to_string() != *HOST)
        .collect()
}
