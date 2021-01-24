use crate::config::SIGNER;
use anyhow::{anyhow, Result};
use ellipticoin_peerchain_ethereum::eth_address;
use ellipticoin_types::{Address, ADDRESS_LENGTH};
use k256::ecdsa::{recoverable, signature::Signer};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    array::TryFromSliceError,
    convert::{TryFrom, TryInto},
};

pub fn sign<S: Serialize>(message: &S) -> [u8; 65] {
    let signature: recoverable::Signature = SIGNER.sign(&serde_cbor::to_vec(message).unwrap());
    k256::ecdsa::signature::Signature::as_bytes(&signature)
        .try_into()
        .unwrap()
}

pub fn sign_eth<S: Serialize>(message: &S) -> [u8; 65] {
    let signature: recoverable::Signature =
        SIGNER.sign(&serde_eth::to_vec_packed(message).unwrap());
    let mut signature: [u8; 65] = k256::ecdsa::signature::Signature::as_bytes(&signature)
        .try_into()
        .unwrap();
    signature[64] = signature[64] + 27;
    signature
}

pub fn recover(message: &[u8], signature_bytes_slice: &[u8]) -> Result<Address> {
    let signature_bytes = signature_bytes_slice.to_vec();
    // See: https://eips.ethereum.org/EIPS/eip-155
    // signature_bytes[SIGNATURE_LENGTH - 1] -= 27;
    let signature = recoverable::Signature::try_from(&signature_bytes[..])
        .map_err(|err| anyhow!(err.to_string()))?;
    let public_key = signature
        .recover_verify_key(&message)
        .map_err(|err| anyhow!(err.to_string()))?;
    eth_address(public_key)[..ADDRESS_LENGTH]
        .try_into()
        .map_err(|e: TryFromSliceError| anyhow!(e.to_string()))
}

pub fn sha256(message: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(message);
    hasher.finalize().into()
}
