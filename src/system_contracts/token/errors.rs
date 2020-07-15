pub use wasm_rpc::error::Error;

lazy_static! {
    pub static ref INSUFFICIENT_FUNDS: Error = Error {
        code: 1,
        message: "Insufficient funds".to_string(),
    };
    pub static ref INSUFFICIENT_ALLOWANCE: Error = Error {
        code: 2,
        message: "Insufficient allowance".to_string(),
    };
}
