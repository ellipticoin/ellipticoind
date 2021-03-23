pub mod db;
pub mod traits;

pub use db::Db;
use std::ops::BitXor;
pub const ADDRESS_LENGTH: usize = 20;
use std::convert::TryInto;
use serde::{Deserialize, Serialize};

#[derive(Copy, Serialize, Deserialize, Debug, Default, PartialEq, Clone, Eq, Hash, PartialOrd, Ord)]
pub struct Address(pub [u8; ADDRESS_LENGTH]);

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl BitXor for Address {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
    Self(self.0.iter().zip(
        rhs.0.iter()
    ).map(|(&a, &b)| a ^ b).collect::<Vec<u8>>().try_into().unwrap())
    }
}
