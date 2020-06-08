use crate::vm::VM;
use metered_wasmi::RuntimeValue;
use serde_cbor::{to_vec, Value};

impl<'a> VM<'a> {
    pub fn call(&mut self, func: &str, arguments: Vec<Value>) -> (Value, Option<u32>) {
        let mut runtime_values: Vec<RuntimeValue> = vec![];
        for arg in arguments {
            let arg_vec = to_vec(&arg).expect("no args");
            match self.write_pointer(arg_vec) {
                Ok(Some(arg_pointer)) => runtime_values.push(arg_pointer),
                Ok(None) => return (0.into(), self.gas),
                Err(trap) => return (trap.to_string().into(), self.gas),
            }
        }
        match self.instance.invoke_export(func, &runtime_values, self) {
            Ok(Some(RuntimeValue::I32(value))) => (
                serde_cbor::from_slice(&self.read_pointer(value)).unwrap(),
                self.gas,
            ),
            Err(trap) => (trap.to_string().into(), self.gas),
            _ => panic!("vm error"),
        }
    }
}
