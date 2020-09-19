use crate::{error::CONTRACT_NOT_FOUND, transaction::TransactionRequest};

#[macro_use]
pub mod macros;
pub mod api;
pub mod bridge;
pub mod ellipticoin;
pub mod exchange;
#[cfg(test)]
pub mod test_api;
pub mod token;

pub fn run<API: ::ellipticoin::API>(
    api: &mut API,
    transaction: TransactionRequest,
) -> serde_cbor::Value {
    let f = match &transaction.contract[..] {
        "Bridge" => bridge::native::call,
        "Ellipticoin" => ellipticoin::native::call,
        "Exchange" => exchange::native::call,
        "Token" => token::native::call,
        _ => {
            return serde_cbor::value::to_value(Err::<(), crate::error::Error>(
                CONTRACT_NOT_FOUND.clone(),
            ))
            .unwrap();
        }
    };
    f(api, &transaction.function, transaction.clone().arguments)
}
