use crate::{
    helpers::sha256,
    system_contracts::{ellipticoin, ellipticoin::Miner},
    types,
};
use async_std::sync::Mutex;
use serde_cbor::from_slice;
use std::{
    collections::HashMap,
    ops::DerefMut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

lazy_static! {
    pub static ref MINERS: async_std::sync::Arc<Mutex<Vec<Miner>>> =
        async_std::sync::Arc::new(Mutex::new(vec![]));
    pub static ref BLOCK_NUMBER: async_std::sync::Arc<AtomicUsize> =
        async_std::sync::Arc::new(AtomicUsize::new(0));
}
impl BLOCK_NUMBER {
    pub fn increment(&self) -> usize {
        let b = Arc::clone(&self);
        b.fetch_add(1, Ordering::SeqCst)
    }
}

pub type Changeset = HashMap<Vec<u8>, Vec<u8>>;
pub struct State {
    pub memory: Memory,
    pub storage: Storage,
    pub memory_changeset: Changeset,
    pub storage_changeset: Changeset,
}

pub struct Memory {
    pub redis: types::redis::Connection,
}

impl Memory {
    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        r2d2_redis::redis::Commands::set::<&[u8], &[u8], ()>(self.redis.deref_mut(), key, value)
            .unwrap()
    }

    pub fn get(&mut self, key: &[u8]) -> Vec<u8> {
        r2d2_redis::redis::Commands::get::<&[u8], Vec<u8>>(self.redis.deref_mut(), key).unwrap()
    }
}

pub struct Storage {
    pub rocksdb: std::sync::Arc<rocksdb::DB>,
}

impl Storage {
    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        self.rocksdb.put(key.to_vec(), value).unwrap()
    }

    pub fn get(&mut self, key: &[u8]) -> Vec<u8> {
        rocksdb::DB::get(&self.rocksdb, key)
            .unwrap()
            .and_then(|value| Some(value))
            .unwrap_or(vec![])
    }
}

impl State {
    pub fn new(memory: Memory, storage: Storage) -> Self {
        let vm_state = Self {
            memory,
            storage,
            memory_changeset: Changeset::new(),
            storage_changeset: Changeset::new(),
        };
        vm_state
    }

    pub fn get_memory(&mut self, key: &[u8]) -> Vec<u8> {
        self.memory.get(key)
    }

    pub fn set_memory(&mut self, key: &[u8], value: &[u8]) {
        self.memory_changeset.insert(key.to_vec(), value.to_vec());
        self.memory.set(key, value);
    }

    pub fn get_storage(&mut self, key: &[u8]) -> Vec<u8> {
        self.storage.get(key)
    }

    pub fn set_storage(&mut self, key: &[u8], value: &[u8]) {
        self.storage_changeset.insert(key.to_vec(), value.to_vec());
        self.storage.set(key, value);
    }

    pub fn block_number(&mut self) -> u32 {
        let bytes = self.get_storage(&vec![ellipticoin::StorageNamespace::BlockNumber as u8]);
        from_slice(&bytes).unwrap_or(0)
    }
}

pub fn db_key(contract: &str, key: &[u8]) -> Vec<u8> {
    [&sha256(contract.as_bytes().to_vec())[..], key].concat()
}
