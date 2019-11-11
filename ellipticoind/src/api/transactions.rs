use super::API;
use vm::redis::Commands;
use vm::Transaction;
use warp::http::StatusCode;
use warp::reply::Reply;

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
