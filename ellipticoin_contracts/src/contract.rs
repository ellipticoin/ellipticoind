use crate::helpers::pad_left;
use ellipticoin_types::{ADDRESS_LENGTH, db::{Backend, Db}};
use serde::{de::DeserializeOwned, Serialize};
use std::convert::TryInto;

pub trait Contract {
    const NAME: Name;

    fn get<K: Into<Vec<u8>>, V: DeserializeOwned + Default, B: Backend>(
        db: &mut Db<B>,
        key: K,
    ) -> V {
        db.get(Self::NAME as u16, key)
    }

    fn insert<K: Into<Vec<u8>>, V: Serialize, B: Backend>(db: &mut Db<B>, key: K, value: V) {
        db.insert(
            Self::NAME as u16,
            key,
            value,
        )
    }

    fn address() -> [u8; ADDRESS_LENGTH] {
        pad_left((Self::NAME as u16).to_be_bytes().to_vec(), ADDRESS_LENGTH)[..ADDRESS_LENGTH]
            .try_into()
            .unwrap()
    }
}

#[repr(u16)]
pub enum Name {
    AMM,
    Bridge,
    Ellipticoin,
    Governance,
    OrderBook,
    System,
    Token,
}
