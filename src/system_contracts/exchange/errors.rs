pub use wasm_rpc::error::Error;

lazy_static! {
    pub static ref POOL_NOT_FOUND: Error = Error {
        code: 1,
        message: "Pool not found. Please create this pool and try again".to_string(),
    };
}
