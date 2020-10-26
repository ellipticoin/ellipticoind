use crate::{
    config::{network_id, verification_key},
    constants::TOKEN_CONTRACT,
    models::transaction::next_nonce,
};
use serde::{Deserialize, Serialize};
use serde_cbor::{from_slice, Value};
use std::convert::TryInto;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TransactionRequest {
    pub nonce: u32,
    pub sender: [u8; 32],
    pub contract: String,
    pub function: String,
    pub arguments: Vec<serde_cbor::Value>,
    pub network_id: u32,
}

impl Default for TransactionRequest {
    fn default() -> Self {
        Self {
            network_id: network_id(),
            contract: TOKEN_CONTRACT.clone(),
            sender: verification_key(),
            nonce: 0,
            function: "".to_string(),
            arguments: vec![],
        }
    }
}

impl TransactionRequest {
    pub fn new(contract: String, function: &str, arguments: Vec<Value>) -> Self {
        let transaction = Self {
            contract,
            nonce: next_nonce(verification_key().to_vec()),
            function: function.to_string(),
            arguments,
            ..Default::default()
        };
        transaction
    }
}

impl From<crate::models::Transaction> for TransactionRequest {
    fn from(transaction: crate::models::Transaction) -> TransactionRequest {
        TransactionRequest {
            network_id: transaction.network_id as u32,
            sender: transaction.sender[..].try_into().unwrap(),
            arguments: from_slice(&transaction.arguments).unwrap(),
            contract: transaction.contract,
            function: transaction.function,
            nonce: transaction.nonce as u32,
        }
    }
}
