pub use wasm_rpc::error::Error;

pub const SENDER_IS_NOT_THE_WINNER: Error = Error {
    code: 1,
    message: "Sender is not the winner of this block",
};

pub const INVALID_VALUE: Error = Error {
    code: 2,
    message: "Invalid value",
};
pub const INSUFFICIENT_FUNDS: Error = Error {
    code: 3,
    message: "Insufficient funds",
};
