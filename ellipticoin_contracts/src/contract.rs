use crate::helpers::pad_left;
use ellipticoin_types::ADDRESS_LENGTH;
use serde::{de::DeserializeOwned, Serialize};
use std::convert::TryInto;

pub trait Contract {
    const NAME: Name;

    fn get<K: Into<Vec<u8>>, V: DeserializeOwned + Default, D: ellipticoin_types::DB>(
        db: &mut D,
        key: K,
    ) -> V {
        db.get(Self::NAME as u16, key)
    }

    fn set<K: Into<Vec<u8>>, V: Serialize, D: ellipticoin_types::DB>(db: &mut D, key: K, value: V) {
        db.set(Self::NAME as u16, key, value)
    }

    fn address() -> [u8; ADDRESS_LENGTH] {
        pad_left((Self::NAME as u16).to_be_bytes().to_vec(), ADDRESS_LENGTH)[..ADDRESS_LENGTH]
            .try_into()
            .unwrap()
    }
}
#[repr(u16)]
pub enum Name {
    Bridge,
    Ellipticoin,
    AMM,
    System,
    Token,
}
