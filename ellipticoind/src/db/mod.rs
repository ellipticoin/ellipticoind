pub mod memory_backend;
pub mod sled_backend;

use crate::constants::DB;
use async_std::sync::RwLock;
pub use memory_backend::MemoryBackend;
pub use sled_backend::SledBackend;
use std::iter::Iterator;

#[derive(Debug)]
pub enum Backend {
    Memory(MemoryBackend),
    // SledDb(SledBackend),
}

// impl Backend {
//     pub async fn new_db<'a>() -> ellipticoin_types::db::Db<'a, Backend> {
//         let mut backend = BACKEND.get().unwrap().write().await;
//         ellipticoin_types::Db {backend: &mut backend, transaction_state: Default::default()}
//     }
// }

impl ellipticoin_types::db::Backend for Backend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        match self {
            // Backend::SledDb(sled_db) => sled_db.get(key),
            Backend::Memory(memory_db) => memory_db.get(key),
        }
    }

    fn insert(&mut self, key: &[u8], value: &[u8])
    where
        Self: Sized,
    {
        match self {
            // Backend::SledDb(sled_db) => sled_db.insert(key, value),
            Backend::Memory(memory_db) => memory_db.insert(key, value),
        }
    }

    // fn iter(&self) -> dyn Iterator<Item = (Vec<u8>, Vec<u8>)> {
    //     match self {
    //         // Backend::SledDb(sled_db) => sled_db.insert(key, value),
    //         Backend::Memory(memory_db) => memory_db.iter(),
    //     }
    // }
}

pub async fn initialize() {
    let memory_backend = MemoryBackend::new();
    let backend = Backend::Memory(memory_backend);
    let db = ellipticoin_types::Db {
        backend: backend,
        transaction_state: Default::default(),
    };
    if matches!(DB.set(RwLock::new(db)), Err(_)) {
        panic!("Failed to initialize db");
    };
}
