pub mod memory_backend;
pub mod sled_backend;

use crate::constants::DB;
use std::collections::HashMap;
use async_std::sync::{RwLock, RwLockReadGuard};
pub use memory_backend::MemoryBackend;
pub use sled_backend::SledBackend;
use std::collections::hash_map::Iter;

#[derive(Debug)]
pub enum Backend {
    Memory(MemoryBackend),
    _SledDb(SledBackend),
}

impl ellipticoin_types::db::Backend for Backend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        match self {
            Backend::_SledDb(sled_db) => sled_db.get(key),
            Backend::Memory(memory_db) => memory_db.get(key),
        }
    }

    fn insert(&mut self, key: &[u8], value: &[u8])
    where
        Self: Sized,
    {
        match self {
            Backend::_SledDb(sled_db) => sled_db.insert(key, value),
            Backend::Memory(memory_db) => memory_db.insert(key, value),
        }
    }

    fn all(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        match self {
            Backend::_SledDb(sled_db) => sled_db.all(),
            Backend::Memory(memory_db) => memory_db.all(),
        }
    }
}

// impl Backend {
//     pub fn iter<'a>(self) -> Cursor<'a> {
//         match self {
//             Backend::_SledDb(sled_db) => panic!(""),
// // BackendIterator{
// //                 iter: BackendIteratorType::SledIterator(sled_db.db.iter())
// //             },
//             Backend::Memory(memory_db) => Cursor{
//                 iter: memory_db.state.iter()
// // BackendIteratorType::HashMapIterator(memory_db.state.iter())
//             },
//         }
//     }
// }

struct StoreLock<'a> {
    guard: RwLockReadGuard<'a, HashMap<Vec<u8>, Vec<u8>>>,
}
impl<'a> StoreLock<'a> {
    fn get_cursor(&self) -> Cursor {
        Cursor { iter: self.guard.iter() }
    }
}

struct BackendIterator<'a> {
    iter: BackendIteratorType<'a>,
}

struct Cursor<'a> {
    iter: Iter<'a, Vec<u8>, Vec<u8>>,
}
enum BackendIteratorType<'a>{
    HashMapIterator(Iter<'a, Vec<u8>, Vec<u8>>),
    SledIterator(sled::Iter),
}
//
// pub async fn lock<'a>() -> StoreLock<'a> {
//     let hash=  DB.get().unwrap().read().await;
//     StoreLock{guard: hash}
// }

impl<'a> Iterator for BackendIterator<'a> {
   type Item = (Vec<u8>, Vec<u8>);
   fn next(&mut self) -> Option<<Self as Iterator>::Item> { 
        None
        // match &mut self.iter {
        //   BackendIteratorType::HashMapIterator(mut hash_map_iter) => hash_map_iter.next().map(|(key, value)| (key.clone(), value.clone())),
        //  BackendIteratorType::SledIterator(sled_iter) => sled_iter.next().map(Result::unwrap).map(|(key, value)| (key.to_vec(), value.to_vec())),
        // }
    }
}

impl IntoIterator for Backend {
    type Item = (Vec<u8>, Vec<u8>);
    // fn next(&mut self) -> std::option::Option<<Self as Iterator>::Item> { todo!() }
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Backend::_SledDb(sled_db) => sled_db.into_iter().map(|(key, value)| (key, value)).collect::<Vec<(Vec<u8>, Vec<u8>)>>().into_iter(),
            Backend::Memory(memory_db) => memory_db.into_iter().map(|(key, value)| (key, value)).collect::<Vec<(Vec<u8>, Vec<u8>)>>().into_iter()
        }
    }
}

pub async fn initialize() {
    let memory_backend = MemoryBackend::new();
    let backend = Backend::Memory(memory_backend);
    let db = ellipticoin_types::Db {
        backend: backend,
        transaction_state: Default::default(),
    };
    // for (key, value) in db {
    //     println!("{:?}", base64::encode(key));
    // }
    if matches!(DB.set(RwLock::new(db)), Err(_)) {
        panic!("Failed to initialize db");
    };
}
