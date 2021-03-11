pub mod memory_backend;
pub mod sled_backend;

use crate::constants::DB;
use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use indicatif::ProgressBar;
pub use memory_backend::MemoryBackend;
use serde_cbor::Deserializer;
pub use sled_backend::SledBackend;
use std::fs::File;
use std::path::Path;

#[derive(Debug)]
pub enum Backend {
    Memory(MemoryBackend),
    Sled(SledBackend),
}

impl ellipticoin_types::db::Backend for Backend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        match self {
            Backend::Sled(sled_db) => sled_db.get(key),
            Backend::Memory(memory_db) => memory_db.get(key),
        }
    }

    fn insert(&mut self, key: &[u8], value: &[u8])
    where
        Self: Sized,
    {
        match self {
            Backend::Sled(sled_db) => sled_db.insert(key, value),
            Backend::Memory(memory_db) => memory_db.insert(key, value),
        }
    }

    fn all(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        match self {
            Backend::Sled(sled_db) => sled_db.all(),
            Backend::Memory(memory_db) => memory_db.all(),
        }
    }

    fn flush(&mut self) {
        match self {
            Backend::Sled(sled_db) => sled_db.flush(),
            Backend::Memory(memory_db) => memory_db.flush(),
        }
    }
}
impl Backend {
    fn dump(&self) {
        println!("\nDumping state...");
        let file = File::create("state-dump.cbor").unwrap();
        let pb = ProgressBar::new(0);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
                .progress_chars("=> "),
        );
        match self {
            Backend::Sled(sled_db) => {
                let state_length = sled_db.db.len();
                pb.set_length(state_length as u64);
                for key_value in sled_db.db.into_iter().map(|v| {
                    let (key, value) = v.unwrap();
                    (key.to_vec(), value.to_vec())
                }) {
                    pb.inc(1);
                    serde_cbor::to_writer(&file, &key_value).unwrap();
                }
                pb.finish();
            }
            Backend::Memory(memory_db) => {
                let state_length = memory_db.state.len();
                pb.set_length(state_length as u64);
                for key_value in &memory_db.state {
                    pb.inc(1);
                    serde_cbor::to_writer(&file, &key_value).unwrap();
                }
                pb.finish();
            }
        }
    }

    fn verify(&self) {
        println!("Verifying state dump");
        let state_dump_file = File::open("state-dump.cbor").unwrap();
        match self {
            Backend::Sled(sled_db) => {
                let mut key_count = 0;
                for (key, value) in Deserializer::from_reader(&state_dump_file)
                    .into_iter::<(Vec<u8>, Vec<u8>)>()
                    .map(Result::unwrap)
                {
                    // Skip verification of ethereum block number
                    if base64::encode(&key) == "AQAAAA==" {
                        continue;
                    };
                    // println!("{}: {} == {}", base64::encode(&key), base64::encode(&value), base64::encode(sled_db.db.get(&key).unwrap_or(None).map(|v| v.to_vec()).unwrap_or(vec![])));
                    assert!(
                        sled_db
                            .db
                            .get(&key)
                            .expect(&format!(
                                "State verification failed {} != {}",
                                base64::encode(key),
                                base64::encode(&value)
                            ))
                            .unwrap()
                            .to_vec()
                            == value
                    );
                    key_count += 1;
                }

                if key_count == sled_db.db.len() {
                    println!("Verified {} keys", key_count);
                } else {
                    panic!("State dump verification failed")
                }
            }
            Backend::Memory(memory_db) => {
                let mut key_count = 0;
                for (key, value) in Deserializer::from_reader(&state_dump_file)
                    .into_iter::<(Vec<u8>, Vec<u8>)>()
                    .map(Result::unwrap)
                {
                    // Skip verification of ethereum block number
                    if base64::encode(&key) == "AQAAAA==" {
                        continue;
                    };
                    // println!("{}: {} == {}", base64::encode(&key), base64::encode(&value), base64::encode(memory_db.state.get(&key).unwrap()));
                    assert!(
                        memory_db
                            .state
                            .get(&key)
                            .expect(&format!(
                                "State verification failed {} != {}",
                                base64::encode(key),
                                base64::encode(&value)
                            ))
                            .to_vec()
                            == value
                    );
                    key_count += 1;
                }

                if key_count == memory_db.state.len() {
                    println!("Verified state dump");
                } else {
                    panic!("State dump verification failed")
                }
            }
        }
    }
}

pub async fn verify() {
    if Path::new("state-dump.cbor").exists() {
        let lock = lock().await;
        lock.verify();
    }
}
pub async fn dump() {
    let lock = lock().await;
    lock.dump();
}
pub struct StoreLock<'a> {
    pub guard: RwLockWriteGuard<'a, Backend>,
}

pub struct ReadLock<'a> {
    pub guard: RwLockReadGuard<'a, Backend>,
}

pub async fn lock<'a>() -> ReadLock<'a> {
    let backend = DB.get().unwrap().read().await;
    ReadLock { guard: backend }
}
impl<'a> Iterator for StoreLock<'a> {
    type Item = (Vec<u8>, Vec<u8>);
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        None
    }
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
        // println!("guard flushing!!");
        self.guard.flush();
    }

    fn all(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.guard.all()
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

impl Iterator for Backend {
    type Item = (Vec<u8>, Vec<u8>);
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub async fn initialize() {
    let sled_backend = SledBackend::new();
    let backend = Backend::Sled(sled_backend);
    if matches!(DB.set(RwLock::new(backend)), Err(_)) {
        panic!("Failed to initialize db");
    };
}
