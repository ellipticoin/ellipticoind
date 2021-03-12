use ellipticoin_types::db::Backend;
use std::collections::HashMap;
#[derive(Debug)]
pub struct MemoryBackend {
    pub state: HashMap<Vec<u8>, Vec<u8>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            state: Default::default(),
        }
    }
}

impl Backend for MemoryBackend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        self.state.get(key).unwrap_or(&vec![]).to_vec()
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.state.insert(key.to_vec(), value.to_vec());
    }

    fn flush(&mut self) {}
}
