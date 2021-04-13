use crate::transaction::TransactionRequest;
use ellipticoin::Address;
use std::{collections::HashMap, convert::TryInto};

pub struct TestAPI<'a> {
    pub state: &'a mut HashMap<Vec<u8>, Vec<u8>>,
    pub contract: String,
    pub transaction: TransactionRequest,
    pub transaction_state: HashMap<Vec<u8>, Vec<u8>>,
    pub sender: [u8; 32],
    pub caller: Address,
}

impl<'a> TestAPI<'a> {
    pub fn new(
        state: &'a mut HashMap<Vec<u8>, Vec<u8>>,
        sender: [u8; 32],
        contract: String,
    ) -> Self {
        let transaction = TransactionRequest {
            network_id: 0,
            contract: contract.clone(),
            function: "".to_string(),
            arguments: vec![],
            nonce: 0,
            sender,
        };
        Self {
            state,
            contract,
            transaction: transaction.clone(),
            transaction_state: HashMap::new(),
            caller: Address::PublicKey(transaction.sender),
            sender: transaction.sender.try_into().unwrap(),
        }
    }
}
impl<'a> ellipticoin::StateAPI for TestAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.transaction_state
            .get(key)
            .unwrap_or(self.state.get(key).unwrap_or(&vec![]))
            .to_vec()
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.transaction_state.insert(key.to_vec(), value.to_vec());
    }

    fn commit(&mut self) {
        self.state.extend(self.transaction_state.clone());
    }

    fn revert(&mut self) {
        self.transaction_state.clear();
    }
}

impl<'a> ellipticoin::API for TestAPI<'a> {
    fn caller(&self) -> Address {
        self.caller.clone()
    }
}
