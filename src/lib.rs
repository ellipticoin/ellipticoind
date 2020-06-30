#![recursion_limit = "200"]
extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate sha2;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate lazy_static;

pub mod config;
pub mod sub_commands;

mod api;
mod block_broadcaster;
mod constants;
mod helpers;
mod models;
mod pg;
mod run_loop;
mod schema;
mod start_up;
mod system_contracts;
mod vm;

use crate::{
    config::{get_redis_connection, get_rocksdb},
    models::Block,
};
use async_std::sync::{Arc, Mutex};

lazy_static! {
    pub static ref IS_CURRENT_MINER: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref WEB_SOCKET: Arc<Mutex<api::websocket::Websocket>> =
        Arc::new(Mutex::new(api::websocket::Websocket::new()));
    pub static ref VM_STATE: Arc<Mutex<vm::State>> = {
        let vm_state = vm::State::new(get_redis_connection(), get_rocksdb());
        Arc::new(Mutex::new(vm_state))
    };
    pub static ref CURRENT_BLOCK: Arc<Mutex<Option<Block>>> = Arc::new(Mutex::new(None));
}
