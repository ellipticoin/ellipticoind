pub use wasm_rpc::error::Error;

lazy_static! {
    pub static ref INVALID_ADDRESS_LENGTH: Error = Error {
        code: 4,
        message: "Addresses must be 32 bytes long".to_string(),
    };
}
