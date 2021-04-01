#![allow(warnings)]
#[macro_use]
extern crate hex_literal;
#[macro_use]
extern crate lazy_static;

extern crate ellipticoin_contracts;
extern crate ellipticoin_types;
extern crate hex;
extern crate num_bigint;
extern crate num_traits;
extern crate rand;
extern crate sha2;

pub mod constants;
pub mod test_backend;

use constants::actors::ALICE;
use ellipticoin_contracts::constants::{BASE_FACTOR, BASE_TOKEN_MANTISSA, EXCHANGE_RATE_MANTISSA};
use ellipticoin_contracts::{Ellipticoin, Token};
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use num_bigint::BigInt;
use num_traits::pow;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, env};
pub use test_backend::TestBackend;

pub fn sha256(value: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(value);
    hasher.result().to_vec()
}

pub fn random_bytes(length: usize) -> Vec<u8> {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(length)
        .collect()
}

pub fn generate_hash_onion(layers: usize, center: Vec<u8>) -> Vec<Vec<u8>> {
    let mut onion = vec![center];
    for _ in 1..(layers) {
        onion.push(sha256(onion.last().unwrap().to_vec()));
    }
    onion
}

pub fn new_db() -> Db<TestBackend> {
    let backend = TestBackend::new();
    Db {
        backend,
        transaction_state: Default::default(),
    }
}
pub fn setup<B: Backend>(db: &mut Db<B>, balances: HashMap<Address, Vec<(u64, Address)>>) {
    Token::set_base_token_exchange_rate(
        db,
        pow(
            BigInt::from(10),
            BASE_TOKEN_MANTISSA + EXCHANGE_RATE_MANTISSA,
        ),
    );
    for (address, balances) in balances.iter() {
        for (balance, token) in balances.iter() {
            Token::set_balance(db, *address, *token, *balance);
        }
    }
    db.commit()
}
