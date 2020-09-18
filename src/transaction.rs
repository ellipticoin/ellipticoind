use crate::{
    config::{keypair, network_id, public_key},
    constants::TOKEN_CONTRACT,
    helpers::random,
};
use ed25519_dalek::{PublicKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use serde_cbor::{to_vec, Value};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub nonce: u32,
    pub sender: [u8; 32],
    pub contract: String,
    pub function: String,
    pub arguments: Vec<serde_cbor::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
    pub network_id: u32,
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            network_id: network_id(),
            contract: TOKEN_CONTRACT.clone(),
            sender: public_key(),
            nonce: 0,
            function: "".to_string(),
            arguments: vec![],
            signature: None,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CompletedTransaction {
    pub network_id: u32,
    pub contract: String,
    pub sender: [u8; 32],
    pub nonce: u32,
    pub function: String,
    pub arguments: Vec<Value>,
    pub return_value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(contract: String, function: &str, arguments: Vec<Value>) -> Self {
        let mut transaction = Self {
            contract,
            nonce: random(),
            function: function.to_string(),
            arguments,
            ..Default::default()
        };

        transaction.sign();
        transaction
    }

    pub fn sign(&mut self) {
        let transaction = self.clone();
        let signature = keypair().sign(&to_vec(&transaction).unwrap());
        self.signature = Some(signature.to_bytes().to_vec());
    }

    pub fn complete(&self, return_value: Value) -> CompletedTransaction {
        CompletedTransaction {
            network_id: self.network_id,
            contract: self.contract.clone(),
            sender: self.sender.clone(),
            nonce: self.nonce,
            function: self.function.clone(),
            arguments: self.arguments.clone(),
            return_value: return_value,
            signature: self.signature.clone(),
        }
    }

    pub fn valid_signature(&self) -> bool {
        if self.signature.is_none() {
            return false;
        };
        let public_key = match PublicKey::from_bytes(&self.sender) {
            Ok(signature) => signature,
            _ => return false,
        };
        let mut signature_bytes = [0; 64];
        signature_bytes.clone_from_slice(&self.signature.clone().unwrap());
        let signature = Signature::new(signature_bytes);
        let mut transaction_without_signature = self.clone();
        transaction_without_signature.signature = None;
        public_key
            .verify(&to_vec(&transaction_without_signature).unwrap(), &signature)
            .is_ok()
    }
}
