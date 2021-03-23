use anyhow::{anyhow, Result};
use ed25519_zebra::VerificationKey;
use sha2::{Digest, Sha256};
use std::convert::{TryFrom, TryInto};

pub fn sha256(message: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(message);
    hasher.finalize().into()
}

pub fn ed25519_verify(message: &[u8], verification_key: &[u8], signature: &[u8]) -> Result<()> {
    VerificationKey::try_from(verification_key)
        .and_then(|vk| vk.verify(&signature[..64].try_into()?, message))
        .map_err(|_| anyhow!("Invalid Signature"))
}
