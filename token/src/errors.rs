pub use wasm_rpc::error::Error;
lazy_static! {
    pub static ref BLOCK_ALREADY_MINTED: Error = (
        1,
        "this block has already been minted".to_string()
    );
    pub static ref NOT_BLOCK_WINNER: Error = (
        2,
        "sender is not the block winner".into(),
    );
    pub static ref INSUFFICIENT_FUNDS: Error = (
        3,
        "insufficient funds".into(),
    );
}
