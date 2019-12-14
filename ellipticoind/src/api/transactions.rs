use super::API;
use crate::diesel::OptionalExtension;
use crate::schema::transactions::dsl::transactions;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use vm::redis::Commands;
use vm::Transaction;
use warp::http::StatusCode;
use warp::reply::Reply;
use warp::reply::Response;

pub fn show(api: API, transaction_hash: String) -> impl Reply {
    let con = api.db.get().unwrap();
    let transaction = transactions
        .find(base64::decode_config(&transaction_hash, base64::URL_SAFE).unwrap())
        .first::<crate::models::Transaction>(&con)
        .optional()
        .unwrap();

    if let Some(transaction) = transaction {
        Response::new(
            serde_cbor::to_vec(&crate::api::Transaction::from(&transaction))
                .unwrap()
                .into(),
        )
    } else {
        warp::http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(vec![].into())
            .unwrap()
    }
}
pub fn create(api: API, transaction: Transaction) -> impl Reply {
    let mut redis = api.redis.get_connection().unwrap();
    redis
        .rpush::<&str, Vec<u8>, ()>(
            "transactions::pending",
            serde_cbor::to_vec(&transaction).unwrap(),
        )
        .unwrap();

    StatusCode::CREATED
}
