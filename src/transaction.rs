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
    pub function: String,
    pub arguments: Vec<serde_cbor::Value>,
    pub gas_limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
    pub network_id: u32,
    pub contract_address: ([u8; 32], String),
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            network_id: network_id(),
            contract_address: TOKEN_CONTRACT.clone(),
            sender: public_key(),
            nonce: 0,
            function: "".to_string(),
            arguments: vec![],
            gas_limit: u32::MAX,
            signature: None,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CompletedTransaction {
    pub network_id: u32,
    pub contract_address: ([u8; 32], String),
    pub sender: [u8; 32],
    pub nonce: u32,
    pub gas_limit: u32,
    pub gas_used: u32,
    pub function: String,
    pub arguments: Vec<Value>,
    pub return_value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(
        contract_address: ([u8; 32], String),
        function: &str,
        arguments: Vec<Value>,
    ) -> Self {
        let mut transaction = Self {
            contract_address,
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

    pub fn contract_name(&self) -> String {
        self.contract_address.clone().1
    }

    pub fn complete(&self, return_value: Value, gas_left: u32) -> CompletedTransaction {
        CompletedTransaction {
            network_id: self.network_id,
            contract_address: self.contract_address.clone(),
            sender: self.sender.clone(),
            nonce: self.nonce,
            gas_limit: self.gas_limit,
            gas_used: self.gas_limit - gas_left,
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