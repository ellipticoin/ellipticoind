use serde::Serialize;
use serde_cbor::{value::to_value, Value};
use std::collections::HashMap;

lazy_static! {
    pub static ref CONTRACT_NOT_FOUND: Error = Error {
        message: "Contract Not Found".to_string(),
    };
}

#[derive(Serialize, Clone)]
pub struct Error {
    pub message: String,
}

impl From<Error> for Value {
    fn from(error: Error) -> Self {
        let mut error_map: HashMap<&str, &str> = HashMap::new();
        error_map.insert("Err", &error.message);
        to_value(error_map).unwrap()
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error {
            message: error.to_string(),
        }
    }
}
