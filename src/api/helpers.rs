use crate::api::ApiState;
use serde_cbor::de::from_slice;

pub async fn body<T: serde::de::DeserializeOwned>(
    req: &mut tide::Request<ApiState>,
) -> Result<T, serde_cbor::Error> {
    from_slice(&req.body_bytes().await.unwrap())
}
