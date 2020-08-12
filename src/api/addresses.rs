use super::State;
use crate::{
    api::helpers::{base64_decode, to_cbor_response},
    config::get_pg_connection,
    models::highest_nonce,
};
use serde::Serialize;
use tide::{Response, Result};

#[derive(Serialize)]
pub struct Address {
    pub highest_nonce: Option<serde_cbor::Value>,
}

pub async fn show(req: tide::Request<State>) -> Result<Response> {
    let con = get_pg_connection();
    let address: String = req.param("address").unwrap();
    let highest_nonce: Option<i64> = highest_nonce(&con, base64_decode(&address).unwrap());
    Ok(to_cbor_response(&Address {
        highest_nonce: highest_nonce
            .map(|highest_nonce| serde_cbor::Value::Integer(highest_nonce as i128)),
    }))
}
