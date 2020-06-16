use super::ApiState;
use crate::vm::redis::Commands;
use http_service::Body;
use tide::Response;

pub async fn show(req: tide::Request<ApiState>) -> Response {
    let key: String = req.param("key").unwrap();
    let mut redis = req.state().redis.get().unwrap();
    let value = redis
        .get::<Vec<u8>, Vec<u8>>(base64::decode_config(&key, base64::URL_SAFE).unwrap())
        .unwrap();

    Response::new(200).body(Body::from(serde_cbor::to_vec(&value).unwrap()))
}
