use crate::types::Address;
use helpers::db_key;
use wasm_rpc::{
    serde::{de::DeserializeOwned, Serialize},
    serde_cbor::{from_slice, to_vec},
};
pub use wasm_rpc::{pointer, serde_cbor};
pub use wasm_rpc_macros::{export, export_native};

pub trait MemoryAPI {
    fn get(&mut self, key: &[u8]) -> Vec<u8>;
    fn set(&mut self, key: &[u8], value: &[u8]);
}

pub trait StorageAPI {
    fn get(&mut self, key: &[u8]) -> Vec<u8>;
    fn set(&mut self, key: &[u8], value: &[u8]);
}

pub trait API: MemoryAPI + StorageAPI {
    fn caller(&self) -> Address;
    fn get_memory<K: Into<Vec<u8>>, V: DeserializeOwned>(
        &mut self,
        contract: &'static str,
        key: K,
    ) -> Result<V, serde_cbor::Error> {
        from_slice(&MemoryAPI::get(self, &db_key(&contract, &key.into())))
    }

    fn set_memory<K: Into<Vec<u8>>, V: Serialize>(
        &mut self,

        contract: &'static str,
        key: K,
        value: V,
    ) {
        MemoryAPI::set(
            self,
            &db_key(&contract, &key.into()),
            &to_vec(&value).unwrap(),
        )
    }

    fn get_storage<K: Into<Vec<u8>>, V: DeserializeOwned>(
        &mut self,
        contract: &'static str,
        key: K,
    ) -> Result<V, serde_cbor::Error> {
        from_slice(&StorageAPI::get(self, &db_key(&contract, &key.into())))
    }

    fn set_storage<K: Into<Vec<u8>>, V: Serialize>(
        &mut self,
        contract: &'static str,

        key: K,
        value: V,
    ) {
        StorageAPI::set(
            self,
            &db_key(&contract, &key.into()),
            &to_vec(&value).unwrap(),
        )
    }
}
