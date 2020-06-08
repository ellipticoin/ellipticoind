use serde_cbor::Value;
use crate::vm::{Env, State, Transaction};

pub fn is_system_contract(transaction: &Transaction) -> bool {
    transaction.contract_address == [[0; 32].to_vec(), "System".as_bytes().to_vec()].concat()
}

pub fn run(transaction: &Transaction, state: &mut State, env: &Env) -> (u32, Value) {
    match transaction.function.as_str() {
        "create_contract" => create_contract(transaction, state, env),
        _ => (0, Value::Null),
    }
}

pub fn create_contract(transaction: &Transaction, state: &mut State, env: &Env) -> (u32, Value) {
    if let [Value::Text(contract_name), serde_cbor::Value::Bytes(code), serde_cbor::Value::Array(arguments)] =
        &transaction.arguments[..]
    {
        let contract_address = [&transaction.sender, contract_name.as_bytes()].concat();
        state.set_code(&contract_address, code);
        let result = run_constuctor(transaction, state, env, contract_name, arguments);
        result
    } else {
        (0, Value::Null)
    }
}
fn run_constuctor(
    transaction: &Transaction,
    state: &mut State,
    env: &Env,
    contract_name: &str,
    arguments: &Vec<Value>,
) -> (u32, Value) {
    let (result, _gas_left) = Transaction {
        function: "constructor".to_string(),
        arguments: arguments.to_vec(),
        sender: transaction.sender.clone(),
        nonce: transaction.nonce,
        gas_limit: transaction.gas_limit,
        contract_address: [
            transaction.sender.clone(),
            contract_name.as_bytes().to_vec(),
        ]
        .concat(),
    }
    .run(state, env);
    result
}

pub fn transfer(
    transaction: &Transaction,
    amount: u32,
    from: Vec<u8>,
    to: Vec<u8>,
) -> (u32, Value) {
    let arguments = vec![Value::Bytes(to), Value::Integer(amount as i128)];
    let transaction = Transaction {
        function: "transfer".to_string(),
        nonce: 0,
        gas_limit: transaction.gas_limit,
        contract_address: [[0 as u8; 32].to_vec(), "BaseToken".as_bytes().to_vec()].concat(),
        sender: from.clone(),
        arguments: arguments.clone(),
    };

    crate::vm::result::contract_not_found(&transaction)
}
