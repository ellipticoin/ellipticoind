extern crate base64;
use env::Env;
pub use metered_wasmi::{
    isa, FunctionContext, ImportsBuilder, Module, ModuleInstance, NopExternals, RuntimeValue,
};
use result::{self, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use vm::{new_module_instance, VM};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Transaction {
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

pub struct _State {
    pub memory_changeset: crate::state::Changeset,
    pub storage_changeset: crate::state::Changeset,
}

impl Transaction {
    pub fn run(&self, mut state: &mut crate::State, env: &Env) -> (Result, Option<u32>) {
        let code = state.get_code(&self.contract_address);
        if code.len() == 0 {
            return (
                result::contract_not_found(self),
                Some(self.gas_limit as u32),
            );
        }
        if let Ok(instance) = new_module_instance(code) {
            let mut vm = VM {
                instance: &instance,
                env: env,
                state: &mut state,
                transaction: self,
                gas: Some(self.gas_limit as u32),
            };
            vm.call(&self.function, self.arguments.clone())
        } else {
            return (result::invalid_wasm(), Some(self.gas_limit as u32));
        }
    }

    pub fn complete(&self, result: Result) -> CompletedTransaction {
        CompletedTransaction {
            contract_address: self.contract_address.clone(),
            sender: self.sender.clone(),
            nonce: self.nonce.clone(),
            gas_limit: self.gas_limit.clone(),
            function: self.function.clone(),
            arguments: self.arguments.clone(),
            return_value: result.1,
            return_code: result.0,
        }
    }
}
