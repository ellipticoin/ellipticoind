pub use wasm_rpc::error::Error;

lazy_static! {
    pub static ref SENDER_IS_NOT_THE_WINNER: Error = Error {
        code: 1,
        message: "Sender is not the winner of this block".to_string(),
    };
    pub static ref INVALID_VALUE: Error = Error {
        code: 2,
        message: "Invalid value".to_string(),
    };
    pub static ref INSUFFICIENT_FUNDS: Error = Error {
        code: 3,
        message: "Insufficient funds".to_string(),
    };
    pub static ref INSUFFICIENT_ALLOWANCE: Error = Error {
        code: 4,
        message: "Insufficient allowance".to_string(),
    };
    pub static ref BALANCE_ALREADY_UNLOCKED: Error = Error {
        code: 5,
        message: "Balance has already been unlocked".to_string(),
    };
    pub static ref BALANCE_EXCEEDS_THIS_PHASE: Error = Error {
        code: 6,
        message: "Only a total of 1000000 ELC can be unlocked in Phase I".to_string(),
    };
    pub static ref MINER_IS_NOT_WHITELISTED: Error = Error {
        code: 6,
        message: "Only a total of 1000000 ELC can be unlocked in Phase I".to_string(),
    };
}
