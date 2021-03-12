use std::collections::HashMap;
use std::iter::Iterator;

#[derive(Debug)]
pub struct TestBackend {
    pub state: HashMap<Vec<u8>, Vec<u8>>,
}

impl TestBackend {
    pub fn new() -> Self {
        Self {
            state: Default::default(),
        }
    }
}

impl ellipticoin_types::db::Backend for TestBackend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        self.state.get(key).unwrap_or(&vec![]).to_vec()
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.state.insert(key.to_vec(), value.to_vec());
    }

    fn flush(&mut self) {}
}
