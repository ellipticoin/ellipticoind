#[cfg(test)]
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate hex_literal;
#[cfg(test)]
extern crate ellipticoin_test_framework;

pub mod bridge;
pub mod constants;
mod contract;
mod crypto;
mod ellipticoin;
mod exchange;
pub mod hash_onion;
mod helpers;
mod system;
mod token;
mod transaction;
mod types;

pub use bridge::Bridge;
pub use ellipticoin::{Ellipticoin, Miner};
pub use exchange::Exchange;
pub use hash_onion::*;
pub use system::System;
pub use token::Token;
pub use transaction::*;
pub use types::*;
