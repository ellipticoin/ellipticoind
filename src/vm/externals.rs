use crate::vm::{gas_costs, new_module_instance, VM};
use metered_wasmi::{isa, RuntimeArgs, RuntimeValue, TrapKind};
use serde_cbor::{from_slice, to_vec};
use std::str;

pub const CONTRACT_ADDRESS_FUNC_INDEX: usize = 0;
pub const SENDER_FUNC_INDEX: usize = 1;
pub const CALLER_FUNC_INDEX: usize = 2;
pub const GET_MEMORY_FUNC_INDEX: usize = 3;
pub const SET_MEMORY_FUNC_INDEX: usize = 4;
pub const GET_STORAGE_FUNC_INDEX: usize = 5;
pub const SET_STORAGE_FUNC_INDEX: usize = 6;
pub const THROW_FUNC_INDEX: usize = 7;
pub const CALL_FUNC_INDEX: usize = 8;
pub const LOG_WRITE: usize = 9;

impl<'a> VM<'a> {
    pub fn contract_address(&self) -> Vec<u8> {
        self.transaction.contract_address.to_vec()
    }

    pub fn sender(&self) -> Vec<u8> {
        self.transaction.sender.to_vec()
    }

    pub fn caller(&self) -> Vec<u8> {
        self.caller.clone()
    }

    pub fn get_memory(&mut self, key_pointer: i32) -> Result<Vec<u8>, metered_wasmi::TrapKind> {
        let key = self.read_pointer(key_pointer);
        let value = self
            .state
            .get_memory(&self.transaction.contract_address, &key);

        self.use_gas(value.len() as u32 * gas_costs::GET_BYTE_MEMORY)?;
        Ok(value)
    }

    pub fn set_memory(
        &mut self,
        key_pointer: i32,
        value_pointer: i32,
    ) -> Result<Option<RuntimeValue>, metered_wasmi::Trap> {
        let key = self.read_pointer(key_pointer);
        let value = self.read_pointer(value_pointer);
        self.state
            .set_memory(&self.transaction.contract_address, &key, &value);
        self.use_gas(value.len() as u32 * gas_costs::SET_BYTE_MEMORY)?;
        Ok(None)
    }

    pub fn get_storage(&mut self, key_pointer: i32) -> Result<Vec<u8>, metered_wasmi::TrapKind> {
        let key = self.read_pointer(key_pointer);
        let value = self
            .state
            .get_storage(&self.transaction.contract_address, &key);
        self.use_gas(value.len() as u32 * gas_costs::GET_BYTE_STORAGE)?;
        Ok(value)
    }

    pub fn set_storage(
        &mut self,
        key_pointer: i32,
        value_pointer: i32,
    ) -> Result<Option<RuntimeValue>, metered_wasmi::Trap> {
        let key = self.read_pointer(key_pointer);
        let value = self.read_pointer(value_pointer);
        self.use_gas(value.len() as u32 * gas_costs::SET_BYTE_STORAGE)?;
        self.state
            .set_storage(&self.transaction.contract_address, &key, &value);
        Ok(None)
    }

    pub fn external_call(
        &mut self,
        contract_address_pointer: i32,
        function_name_pointer: i32,
        arguments_pointer: i32,
    ) -> Result<Vec<u8>, metered_wasmi::Trap> {
        let contract_address = self.read_pointer(contract_address_pointer);
        let function_name_bytes = self.read_pointer(function_name_pointer);
        let function_name = str::from_utf8(&function_name_bytes).unwrap();
        let arguments = from_slice(&self.read_pointer(arguments_pointer)).unwrap();
        let code = self.state.get_code(&contract_address);
        if code.len() == 0 {
            return Ok(to_vec(&(
                serde_cbor::Value::Integer(0),
                Some(self.transaction.gas_limit as u32),
            ))
            .unwrap());
        }
        let module_instance = new_module_instance(code).unwrap();
        let mut transaction = self.transaction.clone();
        transaction.contract_address = contract_address;
        let mut vm = VM {
            instance: &module_instance,
            caller: &self.transaction.contract_address.clone(),
            state: &mut self.state,
            transaction: &transaction,
            gas: self.gas,
        };
        let (result, gas_left) = vm.call(function_name, arguments);
        let gas_used = self.gas - gas_left;
        self.use_gas(gas_used)?;
        Ok(to_vec(&result).unwrap())
    }

    pub fn log(
        &mut self,
        log_level_pointer: i32,
        log_message_pointer: i32,
    ) -> Result<Option<RuntimeValue>, metered_wasmi::Trap> {
        let _log_level = self.read_pointer(log_level_pointer);
        let message = self.read_pointer(log_message_pointer);
        println!(
            "debug: WebAssembly log: {:?}",
            str::from_utf8(&message).unwrap()
        );

        Ok(None)
    }
}

impl metered_wasmi::Externals for VM<'_> {
    fn use_gas(&mut self, _instruction: &isa::Instruction) -> Result<(), TrapKind> {
        self.use_gas(gas_costs::INSTRUCTION)
    }

    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, metered_wasmi::Trap> {
        match index {
            CONTRACT_ADDRESS_FUNC_INDEX => self.write_pointer(self.contract_address()),
            SENDER_FUNC_INDEX => self.write_pointer(self.sender()),
            CALLER_FUNC_INDEX => self.write_pointer(self.caller()),
            GET_MEMORY_FUNC_INDEX => {
                let value_pointer = self.get_memory(args.nth(0))?;
                self.write_pointer(value_pointer)
            }
            SET_MEMORY_FUNC_INDEX => self.set_memory(args.nth(0), args.nth(1)),
            GET_STORAGE_FUNC_INDEX => {
                let value_pointer = self.get_storage(args.nth(0))?;
                self.write_pointer(value_pointer)
            }
            SET_STORAGE_FUNC_INDEX => self.set_storage(args.nth(0), args.nth(1)),
            THROW_FUNC_INDEX => Ok(None),
            CALL_FUNC_INDEX => {
                let result_bytes = self.external_call(args.nth(0), args.nth(1), args.nth(2))?;
                self.write_pointer(result_bytes)
            }
            LOG_WRITE => self.log(args.nth(0), args.nth(1)),
            _ => panic!("Called an unknown function index: {}", index),
        }
    }
}
