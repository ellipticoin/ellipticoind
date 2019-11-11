#![feature(plugin, rustc_private)]
extern crate heck;
extern crate metered_wasmi;
pub extern crate redis;
pub extern crate rocksdb;
extern crate rustler;
extern crate serde;
extern crate serde_cbor;
extern crate serialize;
extern crate sha3;
extern crate time;

pub mod env;
mod gas_costs;
mod helpers;
pub mod result;
mod transaction;
mod vm;
pub mod state;

pub use state::{State, Changeset};
pub use env::Env;
pub use helpers::right_pad_vec;
pub use metered_wasmi::RuntimeValue;
pub use transaction::{CompletedTransaction, Transaction};
pub use vm::{new_module_instance, VM};
pub use result::*;

pub use metered_wasmi::{ImportsBuilder, Module, ModuleInstance, NopExternals};
pub use redis::{pipe, Client, Commands, ControlFlow, PubSubCommands};
pub use redis::aio::{Connection, ConnectionLike};

pub use rocksdb::ops::Open;
pub use rocksdb::{ReadOnlyDB, DB};
pub use rustler::resource::ResourceArc;
pub use rustler::types::atom::Atom;
pub use rustler::{Decoder, Encoder, NifResult, Term};
