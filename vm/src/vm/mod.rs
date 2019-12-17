extern crate hex;
extern crate serialize;
use env::Env;
use metered_wasmi::{ImportsBuilder, Module, ModuleInstance, ModuleRef, NopExternals};
use transaction::Transaction;
use state::State;

mod call;
mod externals;
mod gas;
mod import_resolver;
mod memory;

pub struct VM<'a> {
    pub instance: &'a ModuleRef,
    pub transaction: &'a Transaction,
    pub state: &'a mut State,
    pub gas: Option<u32>,
    pub env: &'a Env,
}

pub fn new_module_instance(code: Vec<u8>) -> Result<ModuleRef, metered_wasmi::Error> {
    let module = Module::from_buffer(code)?;

    let mut imports = ImportsBuilder::new();
    imports.push_resolver("env", &import_resolver::ImportResolver);
    Ok(ModuleInstance::new(&module, &imports)
        .expect("Failed to instantiate module")
        .run_start(&mut NopExternals)
        .expect("Failed to run start function in module"))
}
