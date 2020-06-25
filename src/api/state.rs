use super::ApiState;
use crate::{
    api::helpers::to_cbor_response,
    config::{get_redis_connection, get_rocksdb},
    constants::{Namespace, TOKEN_CONTRACT},
    vm::{redis::Commands, state::db_key},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tide::{Response, Result};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct State {
    pub memory: HashMap<Vec<u8>, Vec<u8>>,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}
pub async fn show(_req: tide::Request<ApiState>) -> Result<Response> {
    let rocksdb = get_rocksdb();
    let iter = rocksdb.prefix_iterator(db_key(
        &TOKEN_CONTRACT,
        &vec![Namespace::UnlockedEthereumBalances as u8],
    ));
    let storage = iter
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<HashMap<Vec<u8>, Vec<u8>>>();

    let mut redis = get_redis_connection();
    let redis_keys: Vec<Vec<u8>> = redis.keys("*").unwrap_or(vec![]);
    let memory = redis_keys
        .iter()
        .map(|key| {
            let value = redis.get(key.to_vec()).unwrap();
            (key.clone(), value)
        })
        .collect::<HashMap<Vec<u8>, Vec<u8>>>();

    Ok(to_cbor_response(&State {
        memory: memory,
        storage: storage,
    }))
}
