use crate::{
    state::State,
    system_contracts::{self},
    transaction::Transaction,
};
use ellipticoin::Address;
use serde::de::DeserializeOwned;
use serde_cbor::Value;

pub struct NativeAPI<'a> {
    pub state: &'a mut State,
    pub transaction: Transaction,
    pub contract: String,
    pub sender: [u8; 32],
    pub caller: Address,
}

impl<'a> ellipticoin::MemoryAPI for NativeAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_memory(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.set_memory(key, value)
    }
}

impl<'a> ellipticoin::StorageAPI for NativeAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_storage(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.set_storage(key, value)
    }
}

impl<'a> ellipticoin::API for NativeAPI<'a> {
    fn sender(&self) -> [u8; 32] {
        self.sender.clone()
    }
    fn caller(&self) -> Address {
        self.caller.clone()
    }
    fn call<D: DeserializeOwned>(
        &mut self,
        contract: &str,
        function_name: &str,
        arguments: Vec<Value>,
    ) -> Result<D, Box<ellipticoin::wasm_rpc::error::Error>> {
        let mut api = NativeAPI {
            state: &mut self.state,
            contract: contract.to_string(),
            caller: Address::Contract(self.contract.to_string()),
            sender: self.sender,
            transaction: self.transaction.clone(),
        };
        let mut transaction = self.transaction.clone();
        transaction.contract = contract.to_string();
        transaction.arguments = arguments;
        transaction.function = function_name.to_string();
        let return_value: serde_cbor::Value = system_contracts::run2(&mut api, transaction).into();
        Ok(serde_cbor::from_slice(&serde_cbor::to_vec(&return_value).unwrap()).unwrap())
    }
}
