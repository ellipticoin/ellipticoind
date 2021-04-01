pub mod db;
pub mod traits;

pub use db::Db;
use std::ops::{BitXor, Shr};
pub const ADDRESS_LENGTH: usize = 20;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[derive(
    Copy, Serialize, Deserialize, Debug, Default, PartialEq, Clone, Eq, Hash, PartialOrd, Ord,
)]
pub struct Address(pub [u8; ADDRESS_LENGTH]);

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl BitXor for Address {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .iter()
                .zip(rhs.0.iter())
                .map(|(&a, &b)| a ^ b)
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap(),
        )
    }
}
impl Shr<usize> for Address {
    type Output = Self;

    fn shr(self, rhs: usize) -> Self::Output {
        let mut lhs = self.0.clone();
        lhs.rotate_right(rhs);
        Self(lhs)
    }
}
