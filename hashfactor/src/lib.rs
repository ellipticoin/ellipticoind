extern crate num_bigint;
extern crate num_traits;
extern crate sha2;
extern crate rand;
extern crate base64;

use rand::Rng;
use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use num_traits::{ToPrimitive, FromPrimitive};
const NUMERATOR_BYTE_LENGTH: usize = 8;

pub fn hashfactor(data: Vec<u8>, target_number_of_hashes: u64) -> u64 {
    let mut rng = rand::thread_rng();
    let mut nonce = rng.gen_range(0, target_number_of_hashes);
    let data_hash = sha256(data);

    while !is_valid_nonce_hash(&data_hash, nonce, target_number_of_hashes){
        nonce = nonce + 1;
    }

    nonce
}

#[inline(always)]
fn is_valid_nonce_hash(
    data_hash: &[u8],
    nonce: u64,
    target_number_of_hashes: u64,
) -> bool {
    let hash = hash_with_nonce(nonce, &data_hash);
    is_factor_of(first_bytes_as_u64(hash), target_number_of_hashes + 1)
}

#[inline(always)]
fn first_bytes_as_u64(hash: Vec<u8>) -> u64 {
    BigUint::from_bytes_le(&hash[..NUMERATOR_BYTE_LENGTH]).to_u64().unwrap()
}

#[inline(always)]
fn is_factor_of(numerator: u64, denominator: u64) -> bool {
    numerator % denominator == 0
}

fn hash_with_nonce(nonce: u64, data: &[u8]) -> Vec<u8>{
    let nonce_big_uint: BigUint = BigUint::from_u64(nonce).unwrap();
    sha256([
           data.to_vec(),
           nonce_big_uint.to_bytes_le(),
    ].concat())
}

fn sha256(message: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(message);
    hasher.result().to_vec()
}
