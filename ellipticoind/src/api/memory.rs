use crate::api::API;
use vm::redis::Commands;
use warp::reply::Response;

pub fn get_memory(mut api: API, path: String) -> Response {
    let value = api
        .redis
        .get::<Vec<u8>, Vec<u8>>(base64::decode_config(&path, base64::URL_SAFE).unwrap())
        .unwrap();
    Response::new(serde_cbor::to_vec(&value).unwrap().into())
}
