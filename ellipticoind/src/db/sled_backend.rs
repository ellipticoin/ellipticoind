use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug)]
pub struct SledBackend {
    pub db: sled::Db,
}

impl SledBackend {
    pub fn new(path: String) -> Result<Self> {
        Ok(Self {
            db: sled::open(path)?,
        })
    }
}

impl<'a> ellipticoin_types::db::Backend for SledBackend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
       self.db.get(key).unwrap_or(None).map(|v| v.to_vec()).unwrap_or(vec![])
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.db.insert(key.to_vec(), value.to_vec());
    }
}
