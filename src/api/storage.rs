use super::ApiState;
use http_service::Body;
use tide::Response;

pub async fn show(req: tide::Request<ApiState>) -> Response {
    let key: String = req.param("key").unwrap();
    let rocksdb = &req.state().rocksdb;
    let value = rocksdb
        .get(base64::decode_config(&key, base64::URL_SAFE).unwrap())
        .unwrap();

    Response::new(200).body(Body::from(serde_cbor::to_vec(&value).unwrap()))
}
