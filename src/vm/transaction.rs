extern crate base64;
use crate::vm::{
    env::Env,
    error::{Error, CONTRACT_NOT_FOUND},
    new_module_instance, State, VM,
};
pub use metered_wasmi::{
    isa, FunctionContext, ImportsBuilder, Module, ModuleInstance, NopExternals, RuntimeValue,
};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub network_id: u32,
    #[serde(with = "serde_bytes")]
    pub contract_address: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
    pub nonce: u64,
    pub gas_limit: u64,
    pub function: String,
    pub arguments: Vec<Value>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CompletedTransaction {
    pub network_id: u32,
    #[serde(with = "serde_bytes")]
    pub contract_address: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
    pub nonce: u64,
    pub gas_limit: u64,
    pub function: String,
    pub arguments: Vec<Value>,
    pub return_value: Value,
}

impl Transaction {
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
            gas_limit: self.gas_limit,
            function: self.function.clone(),
            arguments: self.arguments.clone(),
            return_value: return_value,
        }
    }
}
