use crate::config::HOST;
use crate::{
    constants::{Namespace, TOKEN_CONTRACT},
    helpers::sha256,
    system_contracts::ellipticoin::Miner,
    types,
};
use serde_cbor::from_slice;
use std::{collections::HashMap, ops::DerefMut};

pub type Changeset = HashMap<Vec<u8>, Vec<u8>>;
pub struct State {
    pub memory: Memory,
    pub storage: Storage,
    pub memory_changeset: Changeset,
    pub storage_changeset: Changeset,
}

pub struct Memory {
    pub redis: types::redis::Connection,
}

impl Memory {
    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        r2d2_redis::redis::Commands::set::<&[u8], &[u8], ()>(self.redis.deref_mut(), key, value)
            .unwrap()
    }

    pub fn get(&mut self, key: &[u8]) -> Vec<u8> {
        r2d2_redis::redis::Commands::get::<&[u8], Vec<u8>>(self.redis.deref_mut(), key).unwrap()
    }
}

pub struct Storage {
    pub rocksdb: std::sync::Arc<rocksdb::DB>,
}

impl Storage {
    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        self.rocksdb.put(key.to_vec(), value).unwrap()
    }

    pub fn get(&mut self, key: &[u8]) -> Vec<u8> {
        rocksdb::DB::get(&self.rocksdb, key)
            .unwrap()
            .and_then(|value| Some(value))
            .unwrap_or(vec![])
    }

    pub fn current_miner(&mut self) -> Option<Miner> {
        let miners: Vec<Miner> =
            from_slice(&self.get(&db_key(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8])))
                .unwrap_or(vec![]);
        miners.first().map(|miner| (*miner).clone())
    }
}

impl State {
    pub fn new(memory: Memory, storage: Storage) -> Self {
        let vm_state = Self {
            memory,
            storage,
            memory_changeset: Changeset::new(),
            storage_changeset: Changeset::new(),
        };
        vm_state
    }

    pub fn get_memory(&mut self, contract_address: &([u8; 32], String), key: &[u8]) -> Vec<u8> {
        self.memory.get(&db_key(contract_address, key))
    }

    pub fn set_memory(&mut self, contract_address: &([u8; 32], String), key: &[u8], value: &[u8]) {
        self.memory_changeset
            .insert(db_key(contract_address, key), value.to_vec());
        self.memory.set(&db_key(contract_address, key), value);
    }

    pub fn get_storage(&mut self, contract_address: &([u8; 32], String), key: &[u8]) -> Vec<u8> {
        self.storage.get(&db_key(contract_address, key))
    }

    pub fn set_storage(&mut self, contract_address: &([u8; 32], String), key: &[u8], value: &[u8]) {
        self.storage_changeset
            .insert(db_key(contract_address, key), value.to_vec());
        self.storage.set(&db_key(contract_address, key), value);
    }

    pub fn current_miner(&mut self) -> Option<Miner> {
        // let miners: Vec<Miner> =
        //     from_slice(&self.get_storage(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]))
        //         .unwrap_or(vec![]);
        // miners.first().map(|miner| (*miner).clone())
        None
    }

    pub fn block_number(&mut self) -> u32 {
        let bytes = self.get_storage(&TOKEN_CONTRACT, &vec![Namespace::BlockNumber as u8]);
        from_slice(&bytes).unwrap_or(0)
    }

    pub async fn peers(&mut self) -> Vec<String> {
        let miners: Vec<Miner> = serde_cbor::from_slice(
            &self.get_storage(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]),
        )
        .unwrap();
        miners
            .iter()
            .map(|miner| miner.host.clone())
            .filter(|host| host.to_string() != *HOST)
            .collect()
    }
}

pub fn db_key(contract_address: &([u8; 32], String), key: &[u8]) -> Vec<u8> {
    [
        &sha256([&contract_address.0[..], contract_address.1.as_bytes()].concat())[..],
        key,
    ]
    .concat()
}
