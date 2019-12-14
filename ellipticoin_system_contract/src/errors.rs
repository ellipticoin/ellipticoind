pub use wasm_rpc::error::{Error, ErrorStruct};
pub const BLOCK_ALREADY_MINTED: ErrorStruct<'static> = Error {
    code: 1,
    message: "this block has already been minted",
};
pub const NOT_BLOCK_WINNER: ErrorStruct<'static> = Error {
    code: 2,
    message: "sender is not the block winner",
};
pub const INSUFFICIENT_FUNDS: ErrorStruct<'static> = Error {
    code: 3,
    message: "insufficient funds",
};

