use indicatif::ProgressBar;
use serde_cbor::Deserializer;
use sled::Batch;
use std::{collections::HashMap, fs::File};

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

    pub fn dump(&self) {
        println!("\nDumping state...");
        let file = File::create("var/state-dump.cbor").unwrap();
        let pb = ProgressBar::new(0);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
                .progress_chars("=> "),
        );
        let state_length = self.db.len();
        pb.set_length(state_length as u64);
        for key_value in self.db.iter().map(|v| {
            let (key, value) = v.unwrap();
            (key.to_vec(), value.to_vec())
        }) {
            pb.inc(1);
            serde_cbor::to_writer(&file, &key_value).unwrap();
        }
        pb.finish();
    }

    pub fn verify(&self) {
        println!("Verifying state dump");
        let state_dump_file = File::open("var/state-dump.cbor").unwrap();
        let mut key_count = 0;
        for (key, value) in Deserializer::from_reader(&state_dump_file)
            .into_iter::<(Vec<u8>, Vec<u8>)>()
            .map(Result::unwrap)
        {
            // Skip verification of ethereum block number
            // if base64::encode(&key) == "AQAAAA==" {
            //     continue;
            // };
            // println!("{}: {} == {}", base64::encode(&key), base64::encode(&value), base64::encode(self.db.get(&key).unwrap_or(None).map(|v| v.to_vec()).unwrap_or(vec![])));
            assert!(
                self.db
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

        if key_count == self.db.len() {
            println!("Verified {} keys", key_count);
        } else {
            panic!("State dump verification failed")
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
    }

    fn flush(&mut self) {
        let mut batch = Batch::default();

        for (key, value) in &self.state {
            batch.insert(key.to_vec(), value.to_vec());
        }
        self.db.apply_batch(batch).unwrap();
    }
}
