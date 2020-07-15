pub trait Backend {
    fn set(&mut self, key: &[u8], value: &[u8]);
    fn get(&mut self, key: &[u8]) -> Vec<u8>;
}
//
// impl Backend for redis::Connection {
//     fn set(&mut self, key: &[u8], value: &[u8]) {
//         redis::Commands::set::<&[u8], &[u8], ()>(self, key, value).unwrap()
//     }
//
//     fn get(&mut self, key: &[u8]) -> Vec<u8> {
//         redis::Commands::get::<&[u8], Vec<u8>>(self, key).unwrap()
//     }
// }
//
impl Backend for std::sync::Arc<rocksdb::DB> {
    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.put(key.to_vec(), value).unwrap()
    }

    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        rocksdb::DB::get(self, key)
            .unwrap()
            .and_then(|value| Some(value))
            .unwrap_or(vec![])
    }
}
