use metered_wasmi::RuntimeValue;
use crate::vm::result;
use serde_cbor::{to_vec, Value};
use crate::vm::vm::VM;

impl<'a> VM<'a> {
    pub fn call(&mut self, func: &str, arguments: Vec<Value>) -> (result::Result, Option<u32>) {
        let mut runtime_values: Vec<RuntimeValue> = vec![];
        for arg in arguments {
            let arg_vec = to_vec(&arg).expect("no args");
            match self.write_pointer(arg_vec) {
                Ok(Some(arg_pointer)) => runtime_values.push(arg_pointer),
                Ok(None) => return (result::vm_panic(), self.gas),
                Err(trap) => return (result::wasm_trap(trap), self.gas),
            }
        }
        match self.instance.invoke_export(func, &runtime_values, self) {
            Ok(Some(RuntimeValue::I32(value))) => {
                (result::from_bytes(self.read_pointer(value)), self.gas)
            }
            Err(metered_wasmi::Error::Trap(trap)) => (result::wasm_trap(trap), self.gas),
            Err(_) => (result::function_does_not_exist(func), self.gas),
            _ => panic!("vm error"),
        }
    }
}
