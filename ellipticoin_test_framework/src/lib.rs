#[macro_use]
extern crate hex_literal;
#[macro_use]
extern crate lazy_static;
extern crate ed25519_dalek;
extern crate hex;
extern crate rand;
extern crate secp256k1;
extern crate sha2;

use self::secp256k1::Message;
use rand::{rngs::OsRng, Rng};
use sha2::{Digest, Sha256};
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

pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let signer = secp256k1::Secp256k1::new();
    let mut rng = OsRng::new().unwrap();
    let (private_key, public_key) = signer.generate_keypair(&mut rng);

    (private_key[..].to_vec(), public_key.serialize().to_vec())
}

pub fn secp256k1_sign_recoverable(message_vec: Vec<u8>, private_key_vec: Vec<u8>) -> Vec<u8> {
    let signer = secp256k1::Secp256k1::new();
    let message = Message::from_slice(&message_vec).unwrap();
    let private_key = secp256k1::SecretKey::from_slice(&private_key_vec).unwrap();
    let signature = signer.sign(&message, &private_key).serialize_compact();
    signature.to_vec()
}
