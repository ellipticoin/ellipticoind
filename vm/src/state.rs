use helpers::right_pad_vec;
use rocksdb::ops::Get;
use redis::Commands;
use std::collections::HashMap;
use std::sync::Arc;

pub type Changeset = HashMap<Vec<u8>, Vec<u8>>;
pub struct State {
    pub redis: redis::Connection,
    pub rocksdb: Arc<rocksdb::DB>,
    pub memory_changeset: Changeset,
    pub storage_changeset: Changeset,
}

impl State {
    pub fn new(
        redis: redis::Connection,
        rocksdb: Arc<rocksdb::DB>,
    ) -> Self {
        let vm_state = Self {
            redis,
            rocksdb,
            memory_changeset: Changeset::new(),
            storage_changeset: Changeset::new(),
        };
        vm_state
    }

    pub fn get_code(&self, contract_address: &[u8]) -> Vec<u8> {
        self.get_storage(contract_address, &vec![])
    }

    pub fn set_code(&mut self, contract_address: &[u8], value: &[u8])  {
        self.set_storage(contract_address, &vec![], value)
    }


    pub fn get_memory(&mut self, contract_address: &[u8], key: &[u8]) -> Vec<u8> {
        self.memory_changeset.get(&namespaced_key(contract_address, key)).unwrap_or(
        &self
            .redis
            .get(namespaced_key(contract_address, key))
            .unwrap_or(vec![])
            ).to_vec()
    }

    pub fn set_memory(&mut self, contract_address: &[u8], key: &[u8], value: &[u8]) {
        self
            .memory_changeset
            .insert(namespaced_key(contract_address, key), value.to_vec());
    }

    pub fn get_storage(&self, contract_address: &[u8], key: &[u8]) -> Vec<u8> {
        self.storage_changeset.get(&namespaced_key(contract_address, key)).unwrap_or(
        &self
            .rocksdb
            .get(namespaced_key(contract_address, key))
            .unwrap().and_then(|value| Some(value.to_vec()))
            .unwrap_or(vec![])
            ).to_vec()
    }

    pub fn set_storage(&mut self, contract_address: &[u8], key: &[u8], value: &[u8])  {
        self
            .storage_changeset
            .insert(namespaced_key(contract_address, key), value.to_vec());
    }

    pub fn commit(&self) {
        //commit here
    }
}

pub fn namespaced_key(contract_address: &[u8], key: &[u8]) -> Vec<u8> {
    [
        right_pad_vec(contract_address.to_vec(), 64, 0),
        key.to_vec(),
    ].concat()
}
