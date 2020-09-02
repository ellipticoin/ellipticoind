use super::{helpers::base64_param, State};
use crate::{
    api::helpers::proxy_get,
    config::{get_rocksdb, public_key},
    helpers::current_miner,
    state::{db_key, Storage},
};
use async_std::task::sleep;
use std::{convert::TryInto, str, time::Duration};
use tide::{http::StatusCode, Body, Response, Result};

pub async fn show(req: tide::Request<State>) -> Result<Response> {
    let contract_name: String = req.param("contract_name")?;
    let contract_owner_bytes = base64_param(&req, "contract_owner")?;
    let contract_address = [
        contract_owner_bytes.clone(),
        contract_name.as_bytes().to_vec(),
    ]
    .concat();
    let key_bytes = base64_param(&req, "key")?;
    for _ in 0..10 {
        if let Ok(res) = get_storage(&req, &contract_address, &key_bytes).await {
            return Ok(res);
        }
        sleep(Duration::from_millis(500)).await;
    }
    get_storage(&req, &contract_address, &key_bytes).await
}

async fn get_storage(
    req: &tide::Request<State>,
    contract_address: &[u8],
    key_bytes: &[u8],
) -> Result<Response> {
    let current_miner = current_miner();
    if current_miner.address.eq(&public_key()) {
        let mut storage = Storage {
            rocksdb: get_rocksdb(),
        };
        let value = storage.get(&db_key(
            &((
                contract_address[0..32].try_into().unwrap(),
                str::from_utf8(&contract_address[32..]).unwrap().to_string(),
            )),
            &key_bytes,
        ));
        let mut res = Response::new(StatusCode::Ok);
        res.set_body(Body::from_bytes(value));
        Ok(res)
    } else {
        proxy_get(req, current_miner.host).await
    }
}
