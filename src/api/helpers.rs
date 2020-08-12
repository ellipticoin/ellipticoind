use crate::api::State;
use serde_cbor::{de::from_slice, to_vec};
use tide::{
    http::{Error, StatusCode},
    Body, Request, Response, Result,
};

pub fn base64_param(req: &Request<State>, key: &str) -> Result<Vec<u8>> {
    base64::decode_config(&req.param::<String>(key)?, base64::URL_SAFE)
        .map_err(|err| Error::new(StatusCode::BadRequest, err))
}

pub fn base64_encode(data: &[u8]) -> String {
    base64::encode_config(&data, base64::URL_SAFE)
}

pub fn base64_decode(string: &str) -> std::result::Result<Vec<u8>, base64::DecodeError> {
    base64::decode_config(&string, base64::URL_SAFE)
}

pub async fn proxy_get(req: &Request<State>, proxy_url: String) -> Result<Response> {
    let mut url = req.url().clone();
    let host = proxy_url.split(":").next().unwrap();
    let port = proxy_url.split(":").last().unwrap().parse().unwrap_or(80);
    url.set_host(Some(&host)).unwrap();
    url.set_port(Some(port)).unwrap();
    let mut surf_res = surf::get(url).await.unwrap();
    let mut res = Response::new(surf_res.status().as_u16());
    let body = surf_res.body_bytes().await?;
    res.set_body(Body::from_bytes(body));
    Ok(res)
}

pub async fn proxy_post(
    req: &Request<State>,
    proxy_url: String,
    body: Vec<u8>,
) -> Result<Response> {
    let mut url = req.url().clone();
    let host = proxy_url.split(":").next().unwrap();
    let port = proxy_url.split(":").last().unwrap().parse().unwrap_or(80);
    url.set_host(Some(&host)).unwrap();
    url.set_port(Some(port)).unwrap();
    let mut surf_res = surf::post(url).body_bytes(&body).await.unwrap();
    let mut res = Response::new(surf_res.status().as_u16());
    let body = surf_res.body_bytes().await?;
    res.set_body(Body::from_bytes(body));
    Ok(res)
}

pub async fn body<T: serde::de::DeserializeOwned>(
    req: &mut tide::Request<State>,
) -> serde_cbor::Result<T> {
    from_slice(&req.body_bytes().await.unwrap())
}

pub fn to_cbor_response<T: serde::ser::Serialize>(response: T) -> Response {
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(Body::from_bytes(to_vec(&response).unwrap()));
    res
}
