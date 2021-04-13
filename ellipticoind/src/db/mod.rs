pub mod memory_backend;
pub mod sled_backend;
use crate::config::address;
use crate::constants::DB;
use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use ellipticoin_contracts::{Bridge, Ellipticoin, Miner, System};
pub use memory_backend::MemoryBackend;
pub use sled_backend::SledBackend;
use std::path::Path;

pub async fn verify() {
    if Path::new("var/state-dump.cbor").exists() {
        let lock = lock().await;
        lock.verify();
    }
}
pub async fn dump() {
    let lock = lock().await;
    lock.dump();
}
pub struct StoreLock<'a> {
    pub guard: RwLockWriteGuard<'a, SledBackend>,
}

pub struct ReadLock<'a> {
    pub guard: RwLockReadGuard<'a, SledBackend>,
}

pub async fn lock<'a>() -> ReadLock<'a> {
    let backend = DB.get().unwrap().read().await;
    ReadLock { guard: backend }
}

impl ellipticoin_types::db::Backend for StoreLock<'_> {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        self.guard.get(key)
    }

    fn insert(&mut self, key: &[u8], value: &[u8])
    where
        Self: Sized,
    {
        self.guard.insert(key, value)
    }

    fn flush(&mut self) {
        self.guard.flush();
    }
}

impl ellipticoin_types::db::Backend for ReadLock<'_> {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        self.guard.get(key)
    }

    fn insert(&mut self, _key: &[u8], _value: &[u8])
    where
        Self: Sized,
    {
        panic!("Database lock is read only");
    }

    fn flush(&mut self) {
        panic!("Database lock is read only");
    }
}
impl<'a> ReadLock<'a> {
    pub fn dump(&self) {
        self.guard.dump()
    }

    pub fn verify(&self) {
        self.guard.verify()
    }
}

#[macro_export]
macro_rules! aquire_db_write_lock {
    () => {{
        let backend = DB.get().unwrap().write().await;
        let store_lock = crate::db::StoreLock { guard: backend };
        ellipticoin_types::Db {
            backend: store_lock,
            transaction_state: Default::default(),
        }
    }};
}

#[macro_export]
macro_rules! aquire_db_read_lock {
    () => {{
        let backend = DB.get().unwrap().read().await;
        let store_lock = crate::db::ReadLock { guard: backend };
        ellipticoin_types::Db {
            backend: store_lock,
            transaction_state: Default::default(),
        }
    }};
}

pub async fn get_hash_onion_layers_left() -> Option<u64> {
    Some(
        get_miners()
            .await
            .iter()
            .find(|miner| miner.address == address())?
            .hash_onion_layers_left,
    )
}

pub async fn get_block_number() -> u64 {
    let mut db = aquire_db_read_lock!();
    System::get_block_number(&mut db)
}

pub async fn get_ethereum_block_number() -> u64 {
    let mut db = aquire_db_read_lock!();
    Bridge::get_ethereum_block_number(&mut db)
}

pub async fn get_miners() -> Vec<Miner> {
    let mut db = aquire_db_read_lock!();
    Ellipticoin::get_miners(&mut db)
}

pub async fn get_current_miner() -> Option<Miner> {
    get_miners().await.first().cloned()
}

pub async fn flush() {
    let mut db = aquire_db_write_lock!();
    db.flush();
}

pub async fn initialize() {
    let sled_backend = SledBackend::new();
    if matches!(DB.set(RwLock::new(sled_backend)), Err(_)) {
        panic!("Failed to initialize db");
    };
}
