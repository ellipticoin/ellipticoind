use super::{views::Transaction, ApiState};
use crate::{diesel::OptionalExtension, models, schema::transactions::dsl::transactions};
use diesel::{QueryDsl, RunQueryDsl};

use crate::{api::helpers::body, models::TransactionPool, network::Message};
use http_service::Body;

use tide::Response;

pub async fn show(req: tide::Request<ApiState>) -> Response {
    let con = req.state().db.get().unwrap();
    let transaction_hash: String = req.param("transaction_hash").unwrap();
    let transaction = transactions
        .find(base64::decode_config(&transaction_hash, base64::URL_SAFE).unwrap())
        .first::<models::Transaction>(&con)
        .optional()
        .unwrap();

    if let Some(transaction) = transaction {
        Response::new(200).body(Body::from(
            serde_cbor::to_vec(&Transaction::from(transaction)).unwrap(),
        ))
    } else {
        Response::new(404)
    }
}

pub async fn create(mut req: tide::Request<ApiState>) -> Response {
    let transaction = match body(&mut req).await {
        Ok(transaction) => transaction,
        Err(_) => return Response::new(400),
    };

    TransactionPool::add(&transaction);
    Response::new(201)
}

pub async fn broadcast(mut req: tide::Request<ApiState>) -> Response {
    let transaction: crate::vm::Transaction = match body(&mut req).await {
        Ok(transaction) => transaction,
        Err(e) => {
            println!("{:?}", e);
            return Response::new(400);
        }
    };

    if !transaction.valid_signature() {
        return Response::new(403);
    }

    TransactionPool::add(&transaction);
    let sender_in = req.state().broadcast_sender.clone();
    sender_in.send(Message::Transaction(transaction)).await;
    Response::new(201)
}
