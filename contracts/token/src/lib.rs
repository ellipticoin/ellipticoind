// #![feature(proc_macro_hygiene)]

#[cfg(not(test))]
extern crate ellipticoin;
#[cfg(test)]
extern crate mock_ellipticoin as ellipticoin;

extern crate hex;
extern crate rand;
extern crate sha2;
extern crate tiny_keccak;
extern crate wasm_rpc;
extern crate wasm_rpc_macros;

#[cfg(test)]
extern crate ellipticoin_test_framework;

mod errors;
mod ethereum;
mod hashing;
pub mod token;
