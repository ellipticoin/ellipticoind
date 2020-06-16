use super::ApiState;
use crate::constants::Namespace;
use crate::constants::TOKEN_CONTRACT;
use crate::vm::redis::Commands;
use crate::vm::state::db_key;
use http_service::Body;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tide::Response;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct State {
    pub memory: HashMap<Vec<u8>, Vec<u8>>,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}
pub async fn show(req: tide::Request<ApiState>) -> Response {
    let rocksdb = &req.state().rocksdb;
    let iter = rocksdb.prefix_iterator(db_key(
        &TOKEN_CONTRACT,
        &vec![Namespace::_UnlockedEthereumBalances as u8],
    ));
    let storage = iter
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<HashMap<Vec<u8>, Vec<u8>>>();

    let mut redis = req.state().redis.get().unwrap();
    let redis_keys: Vec<Vec<u8>> = redis.keys("*").unwrap_or(vec![]);
    let memory = redis_keys
        .iter()
        .map(|key| {
            let value = redis.get(key.to_vec()).unwrap();
            (key.clone(), value)
        })
        .collect::<HashMap<Vec<u8>, Vec<u8>>>();

    Response::new(200).body(Body::from(
        serde_cbor::to_vec(&State {
            memory: memory,
            storage: storage,
        })
        .unwrap_or(vec![]),
    ))
}
