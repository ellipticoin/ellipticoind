use crate::{db::Backend, Address, Db};
use anyhow::Result;

pub trait ToKey {
    fn to_key(&self) -> Vec<u8>;
}

impl ToKey for u64 {
    fn to_key(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl ToKey for [u8; 20] {
    fn to_key(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl ToKey for Address {
    fn to_key(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

pub trait Run: core::fmt::Debug {
    fn sender(&self) -> Result<Address>;
    fn run<B: Backend>(&self, db: &mut Db<B>) -> Result<()>;
}
