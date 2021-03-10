use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::iter::IntoIterator;

pub struct Db<B: Backend> {
    pub backend: B,
    pub transaction_state: HashMap<Vec<u8>, Vec<u8>>,
}

impl<B: Backend> Db<B> {
    pub fn get<K: Into<Vec<u8>>, V: DeserializeOwned + Default>(
        &mut self,
        namespace: u16,
        key: K,
    ) -> V
    where
        Self: Sized,
    {
        let full_key = [namespace.to_le_bytes().to_vec(), key.into()].concat();
        let bytes = self
            .transaction_state
            .get(&full_key)
            .unwrap_or(&Backend::get(&mut self.backend, &full_key))
            .to_vec();

        if bytes.len() == 0 {
            Default::default()
        } else {
            serde_cbor::from_slice(&bytes).expect("corrupted db value")
        }
    }

    pub fn insert<K: Into<Vec<u8>>, V: Serialize>(&mut self, namespace: u16, key: K, value: V)
    where
        Self: Sized,
    {
        self.transaction_state.insert(
            [namespace.to_le_bytes().to_vec(), key.into()].concat(),
            serde_cbor::to_vec(&value).unwrap(),
        );
    }

    pub fn insert_raw(&mut self, key: &[u8], value: &[u8])
    where
        Self: Sized,
    {
        Backend::insert(&mut self.backend, &key, &value);
    }

    pub fn commit(&mut self) {
        for (key, value) in &self.transaction_state {
            Backend::insert(&mut self.backend, &key, &value);
        }
        self.transaction_state.clear();
    }

    pub fn revert(&mut self) {
        self.transaction_state.clear();
    }

    pub fn all(&mut self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.backend.all()
    }

    pub fn into_iter(self) -> B::IntoIter {
        self.backend.into_iter()
    }
}

pub trait Backend: Send + Sync + IntoIterator {
    fn get(&self, key: &[u8]) -> Vec<u8>
    where
        Self: Sized;
    fn insert(&mut self, key: &[u8], value: &[u8])
    where
        Self: Sized;
    fn all(&self) -> Vec<(Vec<u8>, Vec<u8>)>
    where
        Self: Sized;
    
}
