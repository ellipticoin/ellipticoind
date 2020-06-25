use crate::{api::views, config::Bootnode, vm::Transaction};
use rand::Rng;
use serde_cbor::Value;
use sha2::{Digest, Sha256};

pub fn sha256(message: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(message);
    hasher.result().to_vec()
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
