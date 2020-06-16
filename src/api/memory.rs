use super::ApiState;
use crate::vm::redis::Commands;
use http_service::Body;
use tide::Response;

pub async fn show(req: tide::Request<ApiState>) -> Response {
    let key: String = req.param("key").unwrap_or("".to_string());
    let mut redis = req.state().redis.get().unwrap();
    if let Ok(value) = redis
        .get::<Vec<u8>, Vec<u8>>(base64::decode_config(&key, base64::URL_SAFE).unwrap())
    {
        Response::new(200).body(Body::from(serde_cbor::to_vec(&value).unwrap()))
    } else {
        Response::new(404)
    }
}
