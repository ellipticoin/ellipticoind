use crate::{
    state::{Memory, State, Storage},
    transaction::TransactionRequest,
};
use ellipticoin::Address;

pub struct NativeAPI<'a> {
    pub state: &'a mut State,
    pub transaction: TransactionRequest,
}

impl<'a> ellipticoin::MemoryAPI for NativeAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_memory(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.set_memory(key, value)
    }
}

impl<'a> ellipticoin::StorageAPI for NativeAPI<'a> {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_storage(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.state.set_storage(key, value)
    }
}

impl<'a> ellipticoin::API for NativeAPI<'a> {
    fn caller(&self) -> Address {
        Address::PublicKey(self.transaction.sender.clone())
    }
}

pub struct ReadOnlyAPI {
    pub state: State,
}
impl ReadOnlyAPI {
    pub fn new(
        rocksdb: std::sync::Arc<rocksdb::DB>,
        redis: crate::types::redis::Connection,
    ) -> Self {
        let memory = Memory { redis };
        let storage = Storage {
            rocksdb: rocksdb.clone(),
        };
        let state = crate::state::State::new(memory, storage);
        Self { state }
    }
}
impl ellipticoin::MemoryAPI for ReadOnlyAPI {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_memory(key)
    }

    fn set(&mut self, _key: &[u8], _value: &[u8]) {
        panic!("tried to write in a read-only context")
    }
}

impl ellipticoin::StorageAPI for ReadOnlyAPI {
    fn get(&mut self, key: &[u8]) -> Vec<u8> {
        self.state.get_storage(key)
    }

    fn set(&mut self, _key: &[u8], _value: &[u8]) {
        panic!("tried to write in a read-only context")
    }
}

impl ellipticoin::API for ReadOnlyAPI {
    fn caller(&self) -> Address {
        panic!("called `caller` in a read-only context")
    }
}
