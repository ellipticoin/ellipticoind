use serde_cbor::Value;
use std::collections::HashMap;
use vm::Changeset;
use vm::Env;
use vm::Transaction;

pub fn is_system_contract(transaction: &Transaction) -> bool {
    transaction.contract_address == [[0; 32].to_vec(), "system".as_bytes().to_vec()].concat()
}

pub fn run(transaction: &Transaction, env: &Env) -> (Changeset, Changeset, (u32, Value)) {
    match transaction.function.as_str() {
        "create_contract" => create_contract(transaction, env),
        _ => (HashMap::new(), HashMap::new(), (0, Value::Null)),
    }
}

pub fn create_contract(
    transaction: &Transaction,
    env: &Env,
) -> (Changeset, Changeset, (u32, Value)) {
    if let [Value::Text(contract_name), serde_cbor::Value::Bytes(_code), serde_cbor::Value::Array(arguments)] =
        &transaction.arguments[..]
    {
        // let balance = memory.get(&[
        //     right_pad_vec([
        //         &[0; 32].to_vec(),
        //         "BaseToken".as_bytes(),
        //     ].concat(), 64, 0),
        //     vec![0],
        //     transaction.sender.clone(),
        // ].concat());
        run_constuctor(transaction, env, contract_name, arguments)
    } else {
        (HashMap::new(), HashMap::new(), (0, Value::Null))
    }
}
fn run_constuctor(
    transaction: &Transaction,
    _env: &Env,
    _contract_name: &str,
    _arguments: &Vec<Value>,
) -> (Changeset, Changeset, (u32, Value)) {
    // let (memory_changeset, storage_changeset, (result, _gas_left)) = Transaction {
    //     function: "constructor".to_string(),
    //     arguments: arguments.to_vec(),
    //     sender: transaction.sender.clone(),
    //     nonce: transaction.nonce,
    //     gas_limit: transaction.gas_limit,
    //     contract_address: [
    //         transaction.sender.clone(),
    //         contract_name.as_bytes().to_vec(),
    //     ]
    //     .concat(),
    // }
    // .run(env);
    // (memory_changeset, storage_changeset, result)
    (
        HashMap::new(),
        HashMap::new(),
        vm::result::contract_not_found(&transaction),
    )
}

pub fn transfer(
    transaction: &Transaction,
    _memory_changeset: Changeset,
    amount: u32,
    from: Vec<u8>,
    to: Vec<u8>,
) -> (Changeset, Changeset, (u32, Value)) {
    let arguments = vec![Value::Bytes(to), Value::Integer(amount as i128)];
    let transaction = Transaction {
        function: "transfer".to_string(),
        nonce: 0,
        gas_limit: transaction.gas_limit,
        contract_address: [[0 as u8; 32].to_vec(), "BaseToken".as_bytes().to_vec()].concat(),
        sender: from.clone(),
        arguments: arguments.clone(),
    };

    (
        HashMap::new(),
        HashMap::new(),
        vm::result::contract_not_found(&transaction),
    )
}
