use super::API;
use vm::redis::Commands;
use vm::Transaction;
use warp::http::StatusCode;
use warp::reply::Reply;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use warp::reply::Response;
use crate::schema::transactions::dsl::transactions;

pub fn show(api: API, transaction_hash: String) -> impl Reply {
    let con = api.db.get().unwrap();
    let transaction = transactions
        .find(base64::decode_config(&transaction_hash, base64::URL_SAFE).unwrap())
        .first::<crate::models::Transaction>(&con)
        .unwrap();

    Response::new(serde_cbor::to_vec(&crate::api::Transaction::from(&transaction)).unwrap().into())
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
