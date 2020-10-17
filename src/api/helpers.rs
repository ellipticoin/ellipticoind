use crate::api::graphql::Error;
use ed25519_zebra::VerificationKey;
use serde::de::DeserializeOwned;
use std::convert::TryFrom;

pub fn validate_signature<D: DeserializeOwned>(cose_bytes: &[u8]) -> Result<D, Error> {
    let sign1: serde_cose::Sign1 = serde_cbor::from_slice(cose_bytes)
        .map_err(|_| Error("invalid COSE message".to_string()))?;
    let key = serde_cose::Key::from(
        VerificationKey::try_from(<[u8; 32]>::try_from(&sign1.kid()[..]).unwrap())
            .map_err(|_| Error("invalid signature".to_string()))?,
    );
    key.verify(&sign1)
        .map_err(|_| Error("invalid signature".to_string()))?;
    serde_cbor::from_slice(&sign1.payload).map_err(|_| Error("invalid CBOR payload".to_string()))
}
