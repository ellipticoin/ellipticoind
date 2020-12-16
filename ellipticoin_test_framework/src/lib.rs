#[macro_use]
extern crate hex_literal;
#[macro_use]
extern crate lazy_static;

extern crate ed25519_dalek;
extern crate ellipticoin;
extern crate ellipticoind;
extern crate hex;
extern crate rand;
extern crate secp256k1;
extern crate sha2;

use constants::actors::{ALICE, ALICES_PRIVATE_KEY};
use ellipticoin::{Token, API};
use ellipticoind::system_contracts::{test_api::TestAPI, token};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, env};

pub mod constants;

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

pub fn setup(
    balances: HashMap<ellipticoin::Address, Vec<(Token, u64)>>,
    state: &mut HashMap<Vec<u8>, Vec<u8>>,
) -> TestAPI {
    env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));

    let mut api = TestAPI::new(state, *ALICE, "Token".to_string());

    for (address, balances) in balances.iter() {
        for (token, balance) in balances.iter() {
            token::set_balance(&mut api, token.clone(), address.clone(), *balance);
        }
    }

    api.commit();
    api
}
