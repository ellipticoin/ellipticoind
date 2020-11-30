use crate::types::Address;
use helpers::db_key;
pub use wasm_rpc::{pointer, serde_cbor};
use wasm_rpc::{
    serde::{de::DeserializeOwned, Serialize},
    serde_cbor::{from_slice, to_vec},
};
pub use wasm_rpc_macros::{export, export_native};

pub trait StateAPI {
    fn get(&mut self, key: &[u8]) -> Vec<u8>;
    fn set(&mut self, key: &[u8], value: &[u8]);
    fn commit(&mut self);
    fn revert(&mut self);
}

pub trait API: StateAPI {
    fn caller(&self) -> Address;

    fn commit(&mut self) {
        StateAPI::commit(self);
    }

    fn revert(&mut self) {
        StateAPI::revert(self);
    }

    fn get_state<K: Into<Vec<u8>>, V: DeserializeOwned>(
        &mut self,
        contract: &'static str,
        key: K,
    ) -> Result<V, serde_cbor::Error> {
        from_slice(&StateAPI::get(self, &db_key(&contract, &key.into())))
    }

    fn set_state<K: Into<Vec<u8>>, V: Serialize>(
        &mut self,

        contract: &'static str,
        key: K,
        value: V,
    ) {
        StateAPI::set(
            self,
            &db_key(&contract, &key.into()),
            &to_vec(&value).unwrap(),
        )
    }
}
