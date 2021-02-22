use std::collections::HashMap;

pub struct TestDB {
    pub state: HashMap<Vec<u8>, Vec<u8>>,
    pub transaction_state: HashMap<Vec<u8>, Vec<u8>>,
}

impl TestDB {
    pub fn new() -> Self {
        Self {
            state: Default::default(),
            transaction_state: Default::default(),
        }
    }
}

impl ellipticoin_types::DB for TestDB {
    fn get_bytes(&mut self, key: &[u8]) -> Vec<u8> {
        self.transaction_state
            .get(key)
            .unwrap_or(self.state.get(key).unwrap_or(&vec![]))
            .to_vec()
    }

    fn set_bytes(&mut self, key: &[u8], value: &[u8]) {
        self.transaction_state.insert(key.to_vec(), value.to_vec());
    }

    fn commit(&mut self) {
        self.state.extend(self.transaction_state.clone());
    }

    fn revert(&mut self) {
        self.transaction_state.clear();
    }
}