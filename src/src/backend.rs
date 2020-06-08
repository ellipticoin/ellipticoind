use state::Changeset;
use r2d2_redis::redis;

pub trait Backend {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn get(&mut self, key: Vec<u8>) -> Vec<u8>;
    fn commit(&mut self, changeset: Changeset) {
        changeset.iter().for_each(|(key, value)| {
            super::backend::Backend::set(self, key.to_vec(), value.to_vec())
        });
    }
}

impl Backend for redis::Connection {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        redis::Commands::set::<Vec<u8>, Vec<u8>, ()>(self, key.to_vec(), value).unwrap()
    }

    fn get(&mut self, key: Vec<u8>) -> Vec<u8> {
        redis::Commands::get::<Vec<u8>, Vec<u8>>(self, key.to_vec()).unwrap()
    }
}

impl Backend for std::sync::Arc<rocksdb::DB> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.put(key.to_vec(), value).unwrap()
    }

    fn get(&mut self, key: Vec<u8>) -> Vec<u8> {
        rocksdb::DB::get(self, key.to_vec())
            .unwrap()
            .and_then(|value| Some(value.to_vec()))
            .unwrap_or(vec![])
    }
}
