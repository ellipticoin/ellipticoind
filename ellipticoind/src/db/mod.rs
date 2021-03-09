pub mod sled_backend;
pub mod memory_backend;

pub use sled_backend::SledBackend;
pub use memory_backend::MemoryBackend;
use async_std::sync::RwLockWriteGuard;
use crate::constants::BACKEND;

#[derive(Debug)]
pub enum Backend {
    Memory(MemoryBackend),
    SledDb(SledBackend),
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
            Backend::SledDb(sled_db) => sled_db.get(key),
            Backend::Memory(memory_db) => memory_db.get(key),
        }
    }

    fn insert(&mut self, key: &[u8], value: &[u8])
    where
        Self: Sized {
        match self {
            Backend::SledDb(sled_db) => sled_db.insert(key, value),
            Backend::Memory(memory_db) => memory_db.insert(key, value),
        }
    }
    
}
