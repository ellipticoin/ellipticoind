use crate::vm::{backend::Backend, helpers::zero_pad_vec, redis};
use std::{collections::HashMap, sync::Arc};

pub type Changeset = HashMap<Vec<u8>, Vec<u8>>;
pub struct State {
    pub redis: redis::Connection,
    pub rocksdb: Arc<rocksdb::DB>,
    pub memory_changeset: Changeset,
    pub storage_changeset: Changeset,
}

impl State {
    pub fn new(redis: redis::Connection, rocksdb: Arc<rocksdb::DB>) -> Self {
        let vm_state = Self {
            redis,
            rocksdb,
            memory_changeset: Changeset::new(),
            storage_changeset: Changeset::new(),
        };
        vm_state
    }

    pub fn get_code(&mut self, contract_address: &[u8]) -> Vec<u8> {
        self.get_storage(contract_address, &vec![])
    }

    pub fn set_code(&mut self, contract_address: &[u8], value: &[u8]) {
        self.set_storage(contract_address, &vec![], value)
    }

    pub fn get_memory(&mut self, contract_address: &[u8], key: &[u8]) -> Vec<u8> {
        self.memory_changeset
            .get(&db_key(contract_address, key))
            .unwrap_or(
                &r2d2_redis::redis::Commands::get(&mut *self.redis, db_key(contract_address, key))
                    .unwrap(),
            )
            .to_vec()
    }

    pub fn set_memory(&mut self, contract_address: &[u8], key: &[u8], value: &[u8]) {
        self.memory_changeset
            .insert(db_key(contract_address, key), value.to_vec());
    }

    pub fn get_storage(&mut self, contract_address: &[u8], key: &[u8]) -> Vec<u8> {
        self.storage_changeset
            .get(&db_key(contract_address, key))
            .unwrap_or(&self.rocksdb.get(db_key(contract_address, key)))
            .to_vec()
    }

    pub fn set_storage(&mut self, contract_address: &[u8], key: &[u8], value: &[u8]) {
        self.storage_changeset
            .insert(db_key(contract_address, key), value.to_vec());
    }

    pub fn commit(&mut self) {
        self.redis.commit(self.memory_changeset.clone());
        self.rocksdb.commit(self.storage_changeset.clone());
    }
}

pub fn db_key(contract_address: &[u8], key: &[u8]) -> Vec<u8> {
    [zero_pad_vec(contract_address, 255 + 32), key.to_vec()].concat()
}
