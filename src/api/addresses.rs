use super::State;
use crate::models::highest_nonce;
use http_service::Body;
use serde::Serialize;
use tide::Response;

#[derive(Serialize)]
pub struct Address {
    pub highest_nonce: Option<serde_cbor::Value>,
}

pub async fn show(req: tide::Request<State>) -> Response {
    let con = req.state().db.get().unwrap();
    let address: String = req.param("address").unwrap();
    let highest_nonce: Option<i64> = highest_nonce(
        &con,
        base64::decode_config(&address, base64::URL_SAFE).unwrap(),
    );
    Response::new(200).body(Body::from(
        serde_cbor::to_vec(&Address {
            highest_nonce: highest_nonce
                .map(|highest_nonce| serde_cbor::Value::Integer(highest_nonce as i128)),
        })
        .unwrap(),
    ))
}
