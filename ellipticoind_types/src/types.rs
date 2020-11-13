use bytes::Bytes;
use errors::INVALID_ADDRESS_LENGTH;
use std::convert::TryFrom;
use std::convert::TryInto;
use wasm_rpc::error::Error;
use wasm_rpc::serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Token {
    pub issuer: Address,
    pub id: Bytes,
}

impl Into<Vec<u8>> for Token {
    fn into(mut self) -> Vec<u8> {
        [self.issuer.to_vec(), self.id.into_vec()].concat()
    }
}

#[derive(Clone, Hash, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum Address {
    PublicKey([u8; 32]),
    Contract(String),
}

impl From<&str> for Address {
    fn from(contract: &str) -> Self {
        Address::Contract(contract.to_string())
    }
}

impl TryFrom<Vec<u8>> for Address {
    type Error = Box<Error>;
    fn try_from(public_key: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Address::PublicKey(
            public_key[..]
                .try_into()
                .map_err(|_| Box::new(INVALID_ADDRESS_LENGTH.clone()))?,
        ))
    }
}

impl TryFrom<Bytes> for Address {
    type Error = Box<Error>;

    fn try_from(public_key: Bytes) -> Result<Self, Self::Error> {
        Ok(Address::PublicKey(
            public_key.into_vec()[..]
                .try_into()
                .map_err(|_| Box::new(INVALID_ADDRESS_LENGTH.clone()))?,
        ))
    }
}

impl Address {
    pub fn to_vec(&mut self) -> Vec<u8> {
        match self {
            Address::PublicKey(address) => address.to_vec(),
            Address::Contract(name) => name.as_bytes().to_vec(),
        }
    }

    pub fn as_public_key(&mut self) -> Option<[u8; 32]> {
        match self {
            Address::PublicKey(address) => Some(*address),
            _ => None,
        }
    }
}

impl Into<Vec<u8>> for Address {
    fn into(mut self) -> Vec<u8> {
        self.to_vec()
    }
}
