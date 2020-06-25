use super::{views::Transaction, ApiState};
use crate::{
    api::helpers::{body, to_cbor_response},
    config::get_pg_connection,
    diesel::OptionalExtension,
    models,
    schema::transactions::dsl::*,
    vm,
    vm::State,
    CURRENT_BLOCK, VM_STATE,
};
use diesel::prelude::*;
use tide::{http::StatusCode, Redirect, Response, Result};

pub async fn show(req: tide::Request<ApiState>) -> Result<Response> {
    let transaction_hash: String = req.param("transaction_hash").unwrap();
    let transaction = transactions
        .find(base64::decode_config(&transaction_hash, base64::URL_SAFE).unwrap())
        .first::<models::Transaction>(&get_pg_connection())
        .optional()
        .unwrap();

    if let Some(transaction) = transaction {
        Ok(to_cbor_response(&Transaction::from(transaction)))
    } else {
        Ok(Response::new(404))
    }
}

pub async fn create(mut req: tide::Request<ApiState>) -> Result<Response> {
    let transaction: vm::Transaction = match body(&mut req).await {
        Ok(transaction) => transaction,
        Err(_) => return Ok(Response::new(400)),
    };

    if !transaction.valid_signature() {
        return Ok(Response::new(403));
    }

    if State::is_block_winner().await {
        let mut vm_state = VM_STATE.lock().await;
        let current_block = CURRENT_BLOCK.lock().await.as_ref().unwrap().clone();
        let completed_transaction =
            models::Transaction::run(&mut vm_state, &current_block, transaction);
        let transaction_url = format!(
            "/transactions/{}",
            base64::encode_config(&completed_transaction.hash, base64::URL_SAFE)
        );
        Ok(Redirect::see_other(transaction_url).into())
    } else {
        let mut vm_state = VM_STATE.lock().await;
        let current_miner = vm_state.current_miner().unwrap();
        let uri = format!("http://{}/transactions", current_miner.host);
        if surf::post(uri)
            .body_bytes(serde_cbor::to_vec(&transaction).unwrap())
            .await
            .is_err()
        {
            println!("failed posting to http://{}/transactions", "");
            Ok(Response::new(StatusCode::ServiceUnavailable))
        } else {
            Ok(Response::new(StatusCode::Ok))
        }
    }
}
