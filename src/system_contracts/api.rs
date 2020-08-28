use crate::{
    state::State,
    system_contracts::{self, is_system_contract},
    transaction::Transaction,
};
use ellipticoin::Address;
use serde::de::DeserializeOwned;
use serde_cbor::Value;

pub struct NativeAPI<'a> {
    pub state: &'a mut State,
    pub transaction: Transaction,
    pub address: ([u8; 32], String),
    pub sender: [u8; 32],
    pub caller: Address,
}

impl<'a> ellipticoin::MemoryAPI for NativeAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_memory(&self.address, key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.set_memory(&self.address, key, value)
    }
}

impl<'a> ellipticoin::StorageAPI for NativeAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_storage(&self.address, key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.set_storage(&self.address, key, value)
    }
}

impl<'a> ellipticoin::API for NativeAPI<'a> {
    fn contract_address(&self) -> ([u8; 32], String) {
        self.address.clone()
    }
    fn sender(&self) -> [u8; 32] {
        self.sender.clone()
    }
    fn caller(&self) -> Address {
        self.caller.clone()
    }
    fn call<D: DeserializeOwned>(
        &mut self,
        legislator: [u8; 32],
        contract_name: &str,
        function_name: &str,
        arguments: Vec<Value>,
    ) -> Result<D, Box<ellipticoin::wasm_rpc::error::Error>> {
        let mut api = NativeAPI {
            state: &mut self.state,
            address: (legislator, contract_name.to_string()),
            caller: Address::Contract(self.address.clone()),
            sender: self.sender,
            transaction: self.transaction.clone(),
        };
        let mut transaction = self.transaction.clone();
        transaction.contract_address = (legislator, contract_name.to_string());
        transaction.arguments = arguments;
        transaction.function = function_name.to_string();
        let return_value: serde_cbor::Value = if is_system_contract(&transaction) {
            system_contracts::run2(&mut api, transaction).into()
        } else {
            // transaction.complete((CONTRACT_NOT_FOUND.clone()).into(), transaction.gas_limit).into()
            panic!();
        };
        Ok(serde_cbor::from_slice(&serde_cbor::to_vec(&return_value).unwrap()).unwrap())
    }
}
