pub use wasm_rpc::error::Error;

lazy_static! {
    pub static ref INVALID_SIGNER: Error = Error {
        code: 1,
        message: "Invalid Signer".to_string(),
    };
}
