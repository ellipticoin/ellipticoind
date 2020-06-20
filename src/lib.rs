extern crate bytes;
extern crate hex;
extern crate mime;
extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate sha2;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate hex_literal;
#[macro_use]
extern crate lazy_static;

pub mod config;
pub mod sub_commands;

mod api;
mod broadcaster;
mod constants;
mod helpers;
mod miner;
mod models;
mod network;
mod pg;
mod run_loop;
mod schema;
mod start_up;
mod system_contracts;
mod transaction_processor;
mod vm;
use crate::models::Block;
use api::app::app as api;
use async_std::sync::Mutex;

lazy_static! {
    pub static ref BEST_BLOCK: async_std::sync::Arc<Mutex<Option<Block>>> =
        async_std::sync::Arc::new(Mutex::new(None));
}
