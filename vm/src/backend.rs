use state::Changeset;
use rocksdb::ops::Put;
use rocksdb::ops::Get;

pub trait Backend {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn get(&mut self, key: Vec<u8>) -> Vec<u8>;
    fn apply(&mut self, changeset: Changeset)  {
        changeset.iter().for_each(|(key, value)| {
            super::backend::Backend::set(self, key.to_vec(), value.to_vec())
        });
    }
}


impl Backend for redis::Client {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        redis::Commands::set::<Vec<u8>, Vec<u8>, ()>(self, key.to_vec(), value).unwrap()
    }

    fn get(&mut self, key: Vec<u8>) -> Vec<u8>{
        redis::Commands::get::<Vec<u8>, Vec<u8>>(self, key.to_vec()).unwrap()
    }
}

impl Backend for std::sync::Arc<rocksdb::DB> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.put(key.to_vec(), value).unwrap()
    }

    fn get(&mut self, key: Vec<u8>) -> Vec<u8>{
        rocksdb::DB::get(self, key.to_vec()).unwrap().unwrap().to_vec()
    }
}
