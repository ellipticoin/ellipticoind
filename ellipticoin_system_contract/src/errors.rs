pub use wasm_rpc::error::{Error, ErrorStruct};
pub const BLOCK_ALREADY_MINTED: ErrorStruct<'static> = Error {
    code: 1,
    message: "this block has already been minted",
};

pub const INSUFFICIENT_FUNDS: ErrorStruct<'static> = Error {
    code: 2,
    message: "insufficient funds",
};

