use crate::{
    constants::SYSTEM_ADDRESS,
    error::CONTRACT_NOT_FOUND,
    transaction::{CompletedTransaction, Transaction},
};

#[macro_use]
pub mod macros;
pub mod api;
pub mod ellipticoin;
pub mod token;

pub fn is_system_contract(transaction: &Transaction) -> bool {
    transaction.contract_address.0 == SYSTEM_ADDRESS
}

pub fn run<API: ::ellipticoin::API>(
    api: &mut API,
    transaction: Transaction,
) -> CompletedTransaction {
    let return_value = run2(api, transaction.clone());
    transaction.complete(return_value, transaction.gas_limit)
}

pub fn run2<API: ::ellipticoin::API>(api: &mut API, transaction: Transaction) -> serde_cbor::Value {
    let f = match &transaction.contract_name()[..] {
        "Ellipticoin" => ellipticoin::call,
        "Token" => token::call,
        _ => {
            return serde_cbor::value::to_value(Err::<(), crate::error::Error>(
                CONTRACT_NOT_FOUND.clone(),
            ))
            .unwrap();
        }
    };
    f(api, &transaction.function, transaction.clone().arguments)
}
