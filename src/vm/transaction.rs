extern crate base64;
use crate::{
    config::{keypair, public_key, OPTS},
    constants::TOKEN_CONTRACT,
    helpers::random,
    vm::{
        env::Env,
        error::{Error, CONTRACT_NOT_FOUND},
        new_module_instance, State, VM,
    },
};
use ed25519_dalek::{PublicKey, Signature};
pub use metered_wasmi::{
    isa, FunctionContext, ImportsBuilder, Module, ModuleInstance, NopExternals, RuntimeValue,
};
use serde::{Deserialize, Serialize};
use serde_cbor::{to_vec, Value};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub nonce: u32,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
    pub function: String,
    pub arguments: Vec<serde_cbor::Value>,
    pub gas_limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "serde_bytes")]
    pub signature: Option<Vec<u8>>,
    pub network_id: u32,
    #[serde(with = "serde_bytes")]
    pub contract_address: Vec<u8>,
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            network_id: OPTS.network_id,
            contract_address: TOKEN_CONTRACT.to_vec(),
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
    #[serde(with = "serde_bytes")]
    pub contract_address: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
    pub nonce: u32,
    pub gas_limit: u64,
    pub function: String,
    pub arguments: Vec<Value>,
    pub return_value: Value,
    pub signature: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(contract_address: Vec<u8>, function: &str, arguments: Vec<Value>) -> Self {
        let transaction = Self {
            contract_address,
            nonce: random(),
            function: function.to_string(),
            arguments,
            ..Default::default()
        };

        transaction.sign()
    }

    pub fn sign(&self) -> Self {
        let mut transaction = self.clone();
        let signature = keypair().sign(&to_vec(&transaction).unwrap());
        transaction.signature = Some(signature.to_bytes().to_vec());
        transaction
    }

    pub fn run(&self, mut state: &mut State, env: &Env) -> (Value, Option<u32>) {
        let code = state.get_code(&self.contract_address);
        if code.len() == 0 {
            return (
                (CONTRACT_NOT_FOUND.clone()).into(),
                Some(self.gas_limit as u32),
            );
        }
        match new_module_instance(code) {
            Ok(instance) => {
                let mut vm = VM {
                    instance: &instance,
                    env: env,
                    state: &mut state,
                    transaction: self,
                    gas: Some(self.gas_limit as u32),
                };
                vm.call(&self.function, self.arguments.clone())
            }
            Err(err) => {
                return (
                    Error {
                        message: err.to_string(),
                    }
                    .into(),
                    Some(self.gas_limit as u32),
                )
            }
        }
    }

    pub fn complete(&self, return_value: Value) -> CompletedTransaction {
        CompletedTransaction {
            network_id: self.network_id,
            contract_address: self.contract_address.clone(),
            sender: self.sender.clone(),
            nonce: self.nonce,
            gas_limit: self.gas_limit as u64,
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
        let signature = match Signature::from_bytes(&self.signature.clone().unwrap()[..]) {
            Ok(signature) => signature,
            _ => return false,
        };
        let mut transaction_without_signature = self.clone();
        transaction_without_signature.signature = None;
        public_key
            .verify(&to_vec(&transaction_without_signature).unwrap(), &signature)
            .is_ok()
    }
}
