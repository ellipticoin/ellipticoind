use rand::Rng;
use sha2::{Digest, Sha256};

pub fn sha256(message: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(message);
    hasher.result().to_vec()
}

pub fn random() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0, u32::max_value() as u64)
}
