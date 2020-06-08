// use rocksdb::ops::Put;
// use rocksdb::ops::Get;
// use vm::zero_pad_vec;

pub struct _Blockchain {
    pub redis: redis::Client,
    pub rocks_db: rocksdb::DB,
}

impl _Blockchain {
    pub fn _new(redis: redis::Client, rocks_db: rocksdb::DB) -> Self {
        Self {
            redis,
            rocks_db,
        }
    }

    pub fn _initalize(&self) {
        // self.rocks_db.put(b"key", b"value");
        // let value = self.rocks_db.get(b"key").unwrap().unwrap().to_vec();
        // self.rocks_db.put(&zero_pad_vec([[0; 32].to_vec(), b"BaseToken".to_vec()].concat(), 64, 0), b"code");
        //
        // let code = self.rocks_db.get(&zero_pad_vec([[0; 32].to_vec(), b"BaseToken".to_vec()].concat(), 64, 0)).unwrap().unwrap().to_vec();
    }
}
