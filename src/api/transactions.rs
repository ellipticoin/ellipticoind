use super::{views::Transaction, State};
use crate::{
    api::{
        helpers::{body, proxy_get, proxy_post, to_cbor_response},
        Message,
    },
    config::{get_pg_connection, public_key},
    diesel::OptionalExtension,
    models,
    schema::transactions::dsl,
    vm, VM_STATE,
};
use diesel::prelude::*;
use futures::channel::oneshot;
use tide::{Redirect, Response, Result};

pub async fn show(req: tide::Request<State>) -> Result<Response> {
    let transaction_hash: String = req.param("transaction_hash").unwrap();
    let current_miner = {
        let mut vm_state = VM_STATE.lock().await;
        vm_state.current_miner().unwrap()
    };
    if current_miner.address.eq(&public_key()) {
        let transaction = dsl::transactions
            .find(base64::decode_config(&transaction_hash, base64::URL_SAFE).unwrap())
            .first::<models::Transaction>(&get_pg_connection())
            .optional()
            .unwrap();

        if let Some(transaction) = transaction {
            Ok(to_cbor_response(&Transaction::from(transaction)))
        } else {
            Ok(Response::new(404))
        }
    } else {
        proxy_get(&req, current_miner.host).await
    }
}

pub async fn create(mut req: tide::Request<State>) -> Result<Response> {
    let transaction: vm::Transaction = match body(&mut req).await {
        Ok(transaction) => transaction,
        Err(_) => return Ok(Response::new(400)),
    };

    if !transaction.valid_signature() {
        return Ok(Response::new(403));
    }

    let current_miner = {
        let mut vm_state = VM_STATE.lock().await;
        vm_state.current_miner().unwrap()
    };
    if current_miner.address.eq(&public_key()) {
        let sender = &req.state().sender;
        let (responder, response) = oneshot::channel();
        sender
            .send(Message::Transaction(transaction, responder))
            .await;
        let completed_transaction = response.await.unwrap();
        let transaction_url = format!(
            "/transactions/{}",
            base64::encode_config(&completed_transaction.hash, base64::URL_SAFE)
        );
        Ok(Redirect::see_other(transaction_url).into())
    } else {
        proxy_post(
            &req,
            current_miner.host,
            serde_cbor::to_vec(&transaction).unwrap(),
        )
        .await
    }
}
