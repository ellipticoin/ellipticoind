use redis::Commands;
use rocksdb::{Options, DB};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::convert::TryInto;

use vm::state::namespaced_key;

pub const SOCKET: &str = "127.0.0.1:3030";
pub const HTTP_HOST: &str = "localhost:3030";
pub const REDIS_URL: &str = "redis://127.0.0.1/";
pub const ROCKSDB_PATH: &str = "./db";
mod constants;
pub use constants::*;

pub fn setup() {
    let db_opts = Options::default();
    DB::destroy(&db_opts, ROCKSDB_PATH).unwrap();
    let redis = redis::Client::open::<&str>(REDIS_URL.into()).unwrap();
    let mut con = redis.get_connection().unwrap();
    redis::cmd("FLUSHALL").query::<()>(&mut con).unwrap();
}

pub fn set_balance(redis_url: &str, address: Vec<u8>, balance: u64) {
    let mut redis = redis::Client::open::<&str>(redis_url.into()).unwrap();
    let contract_address = [SYSTEM_ADDRESS.to_vec(), "Ellipticoin".as_bytes().to_vec()].concat();
    let key = [[0; 1].to_vec(), address.clone()].concat();
    let balance_bytes = unsafe { std::mem::transmute::<u64, [u8; 8]>(balance).to_vec() };
    redis
        .set::<Vec<u8>, Vec<u8>, ()>(namespaced_key(&contract_address, &key), balance_bytes)
        .unwrap();
}

pub async fn get_balance(address: &[u8]) -> u64 {
    let balance_bytes =
        get_memory::<Vec<u8>>(&SYSTEM_CONTRACT, &[vec![0], address.to_vec()].concat()).await;
    unsafe { std::mem::transmute::<[u8; 8], u64>(balance_bytes.as_slice().try_into().unwrap()) }
}

pub async fn get_memory<D: DeserializeOwned>(contract_address: &[u8], key: &[u8]) -> D {
    let memory_key = namespaced_key(contract_address, key);
    get(format!(
        "memory/{}",
        base64::encode_config(&memory_key, base64::URL_SAFE)
    )
    .as_str())
    .await
}

pub async fn get<D: DeserializeOwned>(path: &str) -> D {
    let response = reqwest::get(format!("http://{}/{}", HTTP_HOST, path).as_str())
        .await
        .unwrap();
    let bytes = response.bytes().await.unwrap();
    serde_cbor::from_slice::<D>(&bytes).unwrap()
}

pub async fn post<S: Serialize>(path: &str, payload: S) {
    let url = path_to_url(path);
    reqwest::Client::new()
        .post(&url)
        .body(serde_cbor::to_vec(&payload).unwrap())
        .send()
        .await
        .unwrap();
}

fn path_to_url(path: &str) -> String {
    format!("http://{}{}", HTTP_HOST, path)
}
