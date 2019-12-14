use super::API;
use crate::models::highest_nonce;
use serde::Serialize;
use warp::reply::Reply;
use warp::reply::Response;

#[derive(Serialize)]
pub struct Address {
    pub highest_nonce: Option<serde_cbor::Value>,
}

pub fn show(api: API, address: String) -> impl Reply {
    let con = api.db.get().unwrap();
    let highest_nonce: Option<i64> = highest_nonce(&con, base64::decode(&address).unwrap());
    Response::new(
        serde_cbor::to_vec(&Address {
            highest_nonce: highest_nonce
                .map(|highest_nonce| serde_cbor::Value::Integer(highest_nonce as i128)),
        })
        .unwrap()
        .into(),
    )
}
