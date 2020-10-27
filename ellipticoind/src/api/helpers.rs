use crate::api::graphql::Error;
use crate::api::types::Bytes;
use ed25519_zebra::VerificationKey;
use ellipticoin::PublicKey;
use serde::de::DeserializeOwned;
use serde_cose::Sign1;
use std::convert::TryFrom;

pub fn validate_signature<D: DeserializeOwned>(cose_bytes: &[u8]) -> Result<(D, PublicKey), Error> {
    let sign1: serde_cose::Sign1 = serde_cbor::from_slice(cose_bytes)
        .map_err(|_| Error("invalid COSE message".to_string()))?;
    let signer_pub_key = <PublicKey>::try_from(&sign1.kid()[..])
        .map_err(|_| Error("invalid signature".to_string()))?;
    let serde_key = serde_cose::Key::from(
        VerificationKey::try_from(signer_pub_key)
            .map_err(|_| Error("invalid signature".to_string()))?,
    );
    serde_key
        .verify(&sign1)
        .map_err(|_| Error("invalid signature".to_string()))?;

    serde_cbor::from_slice(&sign1.payload)
        .map(|result| (result, signer_pub_key))
        .map_err(|_| Error("invalid CBOR payload".to_string()))
}

pub fn bytes_from_signed_iterable<'a, I>(signed_data_iterable: I) -> Vec<Bytes>
where
    I: Iterator<Item = &'a Sign1>,
{
    signed_data_iterable
        .map(|t| bytes_from_signed_data(t))
        .collect()
}

pub fn bytes_from_signed_data(signed_data: &Sign1) -> Bytes {
    Bytes::from(serde_cbor::to_vec(signed_data).unwrap())
}
