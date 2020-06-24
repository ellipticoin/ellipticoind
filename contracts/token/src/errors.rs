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

pub const INSUFFICIENT_ALLOWANCE: Error = Error {
    code: 4,
    message: "Insufficient allowance",
};

pub const BALANCE_ALREADY_UNLOCKED: Error = Error {
    code: 5,
    message: "Balance has already been unlocked",
};

pub const BALANCE_EXCEEDS_THIS_PHASE: Error = Error {
    code: 6,
    message: "Only a total of 1000000 ELC can be unlocked in Phase I",
};
