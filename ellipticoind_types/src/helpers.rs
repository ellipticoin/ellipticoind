use sha2::{Digest, Sha256};

pub fn zero_pad_vec(vec: &[u8], len: usize) -> Vec<u8> {
    let mut padded = vec![0; len];
    padded[..vec.len()].clone_from_slice(vec);
    padded
}

pub fn sha256(message: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(message);
    hasher.finalize().into()
}

pub fn db_key(contract_address: &'static str, key: &[u8]) -> Vec<u8> {
    [&sha256(contract_address.as_bytes().to_vec())[..], key].concat()
}
