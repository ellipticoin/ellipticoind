use super::{views::Transaction, State};
use crate::{
    api::{
        helpers::{base64_decode, body, proxy_get, proxy_post, to_cbor_response},
        Message,
    },
    config::{get_pg_connection, public_key},
    diesel::OptionalExtension,
    helpers::current_miner,
    models,
    schema::transactions::dsl,
    transaction,
};
use async_std::task::sleep;
use diesel::prelude::*;
use ellipticoin::Address;
use futures::channel::oneshot;
use std::time::Duration;
use tide::{Redirect, Response, Result};

pub async fn show(req: tide::Request<State>) -> Result<Response> {
    let transaction_hash: String = req.param("transaction_hash").unwrap();
    let current_miner = current_miner();
    if current_miner.address.eq(&Address::PublicKey(public_key())) {
        let transaction = dsl::transactions
            .find(base64_decode(&transaction_hash).unwrap())
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
    let transaction: transaction::Transaction = match body(&mut req).await {
        Ok(transaction) => transaction,
        Err(_err) => {
            println!("{}", _err.to_string());
            return Ok(Response::new(400));
        }
    };
    if !transaction.valid_signature() {
        return Ok(Response::new(403));
    }
    for _ in 0..10 {
        if let Ok(res) = post_transaction(&req, &transaction).await {
            return Ok(res);
        }
        sleep(Duration::from_millis(500)).await;
    }
    post_transaction(&req, &transaction).await
}

async fn post_transaction(
    req: &tide::Request<State>,
    transaction: &transaction::Transaction,
) -> Result<Response> {
    let current_miner = current_miner();
    if current_miner.address.eq(&Address::PublicKey(public_key())) {
        let sender = &req.state().sender;
        let (responder, response) = oneshot::channel();
        sender
            .send(Message::Transaction(transaction.clone(), responder))
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
