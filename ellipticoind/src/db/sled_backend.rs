use sled::Batch;
use std::collections::HashMap;

#[derive(Debug)]
pub struct SledBackend {
    pub state: HashMap<Vec<u8>, Vec<u8>>,
    pub db: sled::Db,
}

impl SledBackend {
    pub fn new() -> Self {
        let db = sled::open("var/db").unwrap();
        db.clear().unwrap();
        Self {
            state: Default::default(),
            db,
        }
    }
}

impl<'a> ellipticoin_types::db::Backend for SledBackend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        self.state
            .get(key)
            .unwrap_or(
                &self
                    .db
                    .get(key)
                    .unwrap_or(None)
                    .map(|v| v.to_vec())
                    .unwrap_or(vec![]),
            )
            .to_vec()
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.state.insert(key.to_vec(), value.to_vec());
        // self.db.insert(key.to_vec(), value.to_vec()).unwrap();
    }

    fn all(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.db
            .iter()
            .map(Result::unwrap)
            .map(|(key, value)| (key.to_vec(), value.to_vec()))
            .collect()
    }
    fn flush(&mut self) {
        let mut batch = Batch::default();

        for (key, value) in &self.state {
            batch.insert(key.to_vec(), value.to_vec());
        }
        self.db.apply_batch(batch).unwrap();
    }
}

impl Iterator for SledBackend {
    type Item = (Vec<u8>, Vec<u8>);
    fn next(&mut self) -> std::option::Option<<Self as Iterator>::Item> {
        todo!()
    }
}
