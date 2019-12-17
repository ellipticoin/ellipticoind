use helpers::right_pad_vec;
use rocksdb::ops::{Get, Put};
use redis::Commands;
use std::collections::HashMap;

pub type Changeset = HashMap<Vec<u8>, Vec<u8>>;
pub struct State {
    pub redis: redis::Connection,
    pub rocksdb: rocksdb::DB,
    pub memory_changeset: Changeset,
    pub storage_changeset: Changeset,
}

impl State {
    pub fn new(
        redis: redis::Connection,
        rocksdb: rocksdb::DB,
        system_contract: Vec<u8>,
    ) -> Self {
        let vm_state = Self {
            redis,
            rocksdb,
            memory_changeset: Changeset::new(),
            storage_changeset: Changeset::new(),
        };
        let token_address = &[[0 as u8;32].to_vec(), b"Ellipticoin".to_vec()].concat();
        vm_state.set_code(token_address, &system_contract.to_vec());
        vm_state
    }

    pub fn get_code(&self, contract_address: &[u8]) -> Vec<u8> {
        self.get_storage(contract_address, &vec![])
    }

    pub fn set_code(&self, contract_address: &[u8], value: &[u8])  {
        self.set_storage(contract_address, &vec![], value)
    }


    pub fn get_memory(&mut self, contract_address: &[u8], key: &[u8]) -> Vec<u8> {
        self
            .redis
            .get(namespaced_key(contract_address, key))
            .unwrap_or(vec![])
    }

    pub fn set_memory(&mut self, contract_address: &[u8], key: &[u8], value: &[u8]) {
        self
            .redis
            .set(namespaced_key(contract_address, key), value)
            .unwrap()
    }

    pub fn get_storage(&self, contract_address: &[u8], key: &[u8]) -> Vec<u8> {
        self
            .rocksdb
            .get(namespaced_key(contract_address, key))
            .unwrap().and_then(|value| Some(value.to_vec()))
            .unwrap_or(vec![])
    }

    pub fn set_storage(&self, contract_address: &[u8], key: &[u8], value: &[u8])  {
        self
            .rocksdb
            .put(namespaced_key(contract_address, key), value).unwrap();
    }
}

pub fn namespaced_key(contract_address: &[u8], key: &[u8]) -> Vec<u8> {
    [
        right_pad_vec(contract_address.to_vec(), 64, 0),
        key.to_vec(),
    ].concat()
}
