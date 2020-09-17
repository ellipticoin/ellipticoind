use super::{helpers::base64_param, State};
use crate::{
    api::helpers::proxy_get,
    config::{get_redis_connection, public_key},
    helpers::current_miner,
    state::{db_key, Memory},
};
use async_std::task::sleep;
use std::{str, time::Duration};
use tide::{http::StatusCode, Body, Response, Result};

pub async fn show(req: tide::Request<State>) -> Result<Response> {
    let contract: String = req.param("contract")?;
    let key_bytes = base64_param(&req, "key")?;
    for _ in 0..10 {
        if let Ok(res) = get_memory(&req, &contract, &key_bytes).await {
            return Ok(res);
        }
        sleep(Duration::from_millis(500)).await;
    }
    get_memory(&req, &contract, &key_bytes).await
}

async fn get_memory(
    req: &tide::Request<State>,
    contract: &str,
    key_bytes: &[u8],
) -> Result<Response> {
    let current_miner = current_miner().await;
    if current_miner.address.eq(&public_key()) {
        let mut memory = Memory {
            redis: get_redis_connection(),
        };
        let value = memory.get(&db_key(
            contract,
            &key_bytes,
        ));
        let mut res = Response::new(StatusCode::Ok);
        res.set_body(Body::from_bytes(value));
        Ok(res)
    } else {
        proxy_get(req, current_miner.host).await
    }
}
