use super::ApiState;
use http_service::Body;
use tide::Response;

pub async fn show(req: tide::Request<ApiState>) -> Response {
    let key: String = req.param("key").unwrap_or("".to_string());
    let rocksdb = &req.state().rocksdb;
    if let Ok(value) = rocksdb
        .get(base64::decode_config(&key, base64::URL_SAFE).unwrap_or(vec![]))
    {
        Response::new(200).body(Body::from(serde_cbor::to_vec(&value).unwrap_or(vec![])))
    } else {
        Response::new(404)
    }
}
