use crate::api::State;
use tide::{
    http::{Error, StatusCode},
    Request, Result,
};

pub fn base64_param(req: &Request<State>, key: &str) -> Result<Vec<u8>> {
    base64::decode_config(&req.param::<String>(key)?, base64::URL_SAFE)
        .map_err(|err| Error::new(StatusCode::BadRequest, err))
}
