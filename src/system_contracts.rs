use crate::{
    constants::TOKEN_CONTRACT,
    helpers::bytes_to_value,
    vm::{self, CompletedTransaction, State, Transaction},
};
use serde_cbor::Value;

pub fn is_system_contract(transaction: &Transaction) -> bool {
    transaction.contract_address == [[0; 32].to_vec(), "System".as_bytes().to_vec()].concat()
}

pub fn run(transaction: &Transaction, state: &mut State) -> CompletedTransaction {
    match transaction.function.as_str() {
        "create_contract" => create_contract(transaction, state),
        _ => transaction.complete(Value::Null, transaction.gas_limit),
    }
}

pub fn create_contract(transaction: &Transaction, state: &mut State) -> CompletedTransaction {
    if let [Value::Text(contract_name), serde_cbor::Value::Bytes(code), serde_cbor::Value::Array(arguments)] =
        &transaction.arguments[..]
    {
        let contract_address = [&transaction.sender, contract_name.as_bytes()].concat();
        state.set_code(&contract_address, code);
        run_constuctor(transaction, state, contract_name, arguments)
    } else {
        transaction.complete(Value::Null, transaction.gas_limit)
    }
}
fn run_constuctor(
    transaction: &Transaction,
    state: &mut State,
    contract_name: &str,
    arguments: &Vec<Value>,
) -> CompletedTransaction {
    Transaction::new(
        [
            transaction.sender.clone(),
            contract_name.as_bytes().to_vec(),
        ]
        .concat(),
        "constructor",
        arguments.to_vec(),
    )
    .run(state)
}

pub async fn _transfer(
    amount: u32,
    from: Vec<u8>,
    to: Vec<u8>,
    state: &mut vm::State,
) -> CompletedTransaction {
    let arguments = vec![
        bytes_to_value(from.clone()),
        bytes_to_value(to.clone()),
        amount.into(),
    ];
    let transfer = Transaction::new(TOKEN_CONTRACT.to_vec(), "transfer_from", arguments.clone());
    transfer.run(state)
}
