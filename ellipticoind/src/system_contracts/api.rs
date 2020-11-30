use crate::transaction::TransactionRequest;
use ellipticoin::Address;
use std::collections::HashMap;

pub struct InMemoryAPI<'a> {
    pub state: &'a mut HashMap<Vec<u8>, Vec<u8>>,
    pub transaction_state: HashMap<Vec<u8>, Vec<u8>>,
    pub transaction: TransactionRequest,
}

impl<'a> InMemoryAPI<'a> {
    pub fn new(
        state: &'a mut async_std::sync::MutexGuard<'_, HashMap<Vec<u8>, Vec<u8>>>,
        transaction_request: Option<TransactionRequest>,
    ) -> InMemoryAPI<'a> {
        InMemoryAPI {
            transaction: transaction_request.unwrap_or(TransactionRequest {
                ..Default::default()
            }),
            transaction_state: HashMap::new(),
            state,
        }
    }
}
impl<'a> ellipticoin::StateAPI for InMemoryAPI<'a> {
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

impl<'a> ellipticoin::API for InMemoryAPI<'a> {
    fn caller(&self) -> Address {
        Address::PublicKey(self.transaction.sender.clone())
    }
}
