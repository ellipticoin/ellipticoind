use serde_cbor::Value;
// use std::intrinsics::transmute;
use transaction::Transaction;

pub type Result = (u32, Value);
pub fn vm_panic() -> Result {
    (1, "vm panic".to_string().into())
}

pub fn wasm_trap(trap: metered_wasmi::Trap) -> Result {
    (1, format!("WebAssembly Trap: {:?}", trap.kind()).into())
}

pub fn contract_not_found(_transaction: &Transaction) -> Result {
    (2, "Contract not found".to_string().into())
}

pub fn invalid_wasm() -> Result {
    (3, "Invalid WebAssembly Code".to_string().into())
}

pub fn to_bytes(result: Result) -> Vec<u8> {
    let return_bytes = serde_cbor::to_vec(&result.1).unwrap();
    [u32::to_le_bytes(result.0).to_vec(), return_bytes].concat()
}
pub fn from_bytes(bytes: Vec<u8>) -> Result {
    if bytes.len() == 0 {
        vm_panic()
    } else {
        (0, serde_cbor::from_slice(&bytes).unwrap())
    }
}
