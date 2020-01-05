#![feature(proc_macro_hygiene)]

#[cfg(not(test))]
extern crate wee_alloc;

#[cfg(not(test))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
extern crate alloc;
#[cfg(not(test))]
extern crate ellipticoin;
#[cfg(test)]
extern crate mock_ellipticoin as ellipticoin;

extern crate tiny_keccak;
extern crate wasm_rpc;
extern crate wasm_rpc_macros;
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate ellipticoin_test_framework;

mod errors;
mod ethereum;
mod issuance;
pub mod token;
