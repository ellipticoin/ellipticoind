use serde::{de::DeserializeOwned, Serialize};

pub trait DB {
    fn get_bytes(&mut self, key: &[u8]) -> Vec<u8>;
    fn set_bytes(&mut self, key: &[u8], value: &[u8]);
    fn commit(&mut self);
    fn revert(&mut self);
    fn get<K: Into<Vec<u8>>, V: DeserializeOwned + Default>(
        &mut self,
        namespace: u16,
        key: K,
    ) -> V {
        let full_key = [namespace.to_le_bytes().to_vec(), key.into()].concat();
        let bytes = self.get_bytes(&full_key);
        if bytes.len() == 0 {
            Default::default()
        } else {
            serde_cbor::from_slice(&bytes).expect("corrupted db value")
        }
    }

    fn set<K: Into<Vec<u8>>, V: Serialize>(&mut self, namespace: u16, key: K, value: V) {
        self.set_bytes(
            &[namespace.to_le_bytes().to_vec(), key.into()].concat(),
            &serde_cbor::to_vec(&value).unwrap(),
        )
    }
}
