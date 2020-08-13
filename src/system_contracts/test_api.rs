use crate::{
    system_contracts::{self, is_system_contract, token},
    transaction::Transaction,
};
use ellipticoin::{Address, Token};
use serde::de::DeserializeOwned;
use serde_cbor::{from_slice, to_vec};
use std::{collections::BTreeMap, convert::TryInto};
pub struct TestState {
    pub memory: BTreeMap<Vec<u8>, Vec<u8>>,
    pub storage: BTreeMap<Vec<u8>, Vec<u8>>,
    pub memory_changeset: BTreeMap<Vec<u8>, Vec<u8>>,
    pub storage_changeset: BTreeMap<Vec<u8>, Vec<u8>>,
}
impl TestState {
    pub fn new() -> Self {
        Self {
            memory: BTreeMap::new(),
            storage: BTreeMap::new(),
            storage_changeset: BTreeMap::new(),
            memory_changeset: BTreeMap::new(),
        }
    }
}
pub struct TestAPI<'a> {
    pub state: &'a mut TestState,
    pub address: ([u8; 32], String),
    pub transaction: Transaction,
    pub sender: [u8; 32],
    pub caller: Address,
}

impl<'a> TestAPI<'a> {
    pub fn new(state: &'a mut TestState, sender: [u8; 32], address: ([u8; 32], String)) -> Self {
        let transaction = Transaction {
            sender,
            ..Default::default()
        };
        Self {
            state,
            address,
            transaction: transaction.clone(),
            caller: Address::PublicKey(transaction.sender),
            sender: transaction.sender.try_into().unwrap(),
        }
    }

    pub fn set_balance(&mut self, token: Token, address: [u8; 32], balance: u64) {
        self.state.memory.insert(
            [
                [token::Namespace::Balances as u8].to_vec(),
                token.into(),
                address.to_vec(),
            ]
            .concat(),
            to_vec(&balance).unwrap(),
        );
    }

    pub fn get_balance(&mut self, token: Token, address: [u8; 32]) -> u64 {
        from_slice::<u64>(
            self.state
                .memory
                .get(
                    &[
                        [token::Namespace::Balances as u8].to_vec(),
                        token.into(),
                        address.to_vec(),
                    ]
                    .concat(),
                )
                .unwrap_or(&vec![]),
        )
        .unwrap_or(0)
    }
}
impl<'a> ellipticoin::MemoryAPI for TestAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.memory.get(key).unwrap_or(&vec![]).to_vec()
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.memory.insert(key.to_vec(), value.to_vec());
    }
}

impl<'a> ellipticoin::StorageAPI for TestAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.storage.get(key).unwrap_or(&vec![]).to_vec()
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.storage.insert(key.to_vec(), value.to_vec());
    }
}

impl<'a> ellipticoin::API for TestAPI<'a> {
    fn contract_address(&self) -> ([u8; 32], String) {
        self.address.clone()
    }
    fn sender(&self) -> [u8; 32] {
        self.sender
    }
    fn caller(&self) -> Address {
        self.caller.clone()
    }
    fn call<D: DeserializeOwned>(
        &mut self,
        legislator: [u8; 32],
        contract_name: &str,
        function_name: &str,
        arguments: Vec<ellipticoin::wasm_rpc::serde_cbor::Value>,
    ) -> Result<D, Box<ellipticoin::wasm_rpc::error::Error>> {
        let mut transaction = self.transaction.clone();
        transaction.contract_address = (legislator, contract_name.to_string());
        transaction.arguments = arguments;
        transaction.function = function_name.to_string();
        let mut api = TestAPI {
            state: &mut self.state,
            address: (legislator, contract_name.to_string()),
            caller: Address::Contract(self.address.clone()),
            sender: self.sender,
            transaction: transaction.clone(),
        };
        let return_value: serde_cbor::Value = if is_system_contract(&transaction) {
            system_contracts::run2(&mut api, transaction).into()
        } else {
            // transaction.complete((CONTRACT_NOT_FOUND.clone()).into(), transaction.gas_limit).into()
            panic!();
        };
        Ok(serde_cbor::from_slice(&serde_cbor::to_vec(&return_value).unwrap()).unwrap())
    }
}
