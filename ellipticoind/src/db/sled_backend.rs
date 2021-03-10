#[derive(Debug)]
pub struct SledBackend {
    pub db: sled::Db,
}

impl<'a> ellipticoin_types::db::Backend for SledBackend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        self.db
            .get(key)
            .unwrap_or(None)
            .map(|v| v.to_vec())
            .unwrap_or(vec![])
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.db.insert(key.to_vec(), value.to_vec()).unwrap();
    }

    fn all(&self) -> Vec<(Vec<u8>, Vec<u8>)>  {
        self.db.iter().map(Result::unwrap).map(|(key, value)| (key.to_vec(), value.to_vec())).collect()
    }
}

impl Iterator for SledBackend {
    type Item = (Vec<u8>, Vec<u8>);
    fn next(&mut self) -> std::option::Option<<Self as Iterator>::Item> { todo!() }
}
