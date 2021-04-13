pub use wasm_rpc::error::Error;

lazy_static! {
    pub static ref POOL_NOT_FOUND: Error = Error {
        code: 1,
        message: "Pool not found. Please create this pool and try again".to_string(),
    };
    pub static ref INSUFFICIENT_BALANCE: Error = Error {
        code: 2,
        message: "Insufficient balance".to_string(),
    };
    pub static ref MAX_SLIPPAGE_EXCEEDED: Error = Error {
        code: 3,
        message: "Max slippage exceeded for this trade. Trade not executed.".to_string(),
    };
    pub static ref POOL_ALREADY_EXISTS: Error = Error {
        code: 4,
        message: "Pool already exists for the provided token. New pool not created.".to_string(),
    };
}
