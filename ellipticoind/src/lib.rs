#![recursion_limit = "256"]
#![feature(async_closure)]
extern crate bytes;
extern crate hashfactor;
extern crate hex;
extern crate mime;
extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate sha2;
extern crate tokio;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod api;
mod constants;
mod helpers;
mod miner;
pub mod models;
mod network;
pub mod schema;
mod system_contracts;
mod transaction_processor;

use crate::constants::TOKEN_CONTRACT;
use crate::miner::{get_best_block, mine_next_block};
use ::network::Server;
use api::app::app as api;
use async_std::prelude::FutureExt;
use async_std::sync::channel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use models::is_next_block;
use std::net::SocketAddr;
use std::sync::Arc;
use vm::rocksdb::ops::Open;
use vm::Backend;

pub async fn run(
    socket: SocketAddr,
    websocket_socket: SocketAddr,
    database_url: &str,
    rocksdb_path: &str,
    redis_url: &str,
    network: Server<Vec<u8>>,
    system_contract: Vec<u8>,
) {
    diesel_migrations::embed_migrations!();
    let db = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));
    embedded_migrations::run(&db).unwrap();
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    let redis = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let mut redis2 = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let mut rocksdb = Arc::new(vm::rocksdb::DB::open_default(rocksdb_path).unwrap());
    let mut api_state = api::State::new(redis, rocksdb.clone(), pg_pool, network.sender.clone());
    let mut vm_state = vm::State::new(redis2.get_connection().unwrap(), rocksdb.clone());
    vm_state.set_code(&TOKEN_CONTRACT.to_vec(), &system_contract.to_vec());
    let (new_block_sender, new_block_receiver) = channel(1);
    diesel::sql_query("TRUNCATE blocks CASCADE")
        .execute(&db)
        .unwrap();
    async_std::task::spawn(api(api_state.clone()).listen(socket));
    async_std::task::spawn(network::handle_messages(
        api_state.clone(),
        network,
        new_block_sender,
    ));
    let mut websocket = api::websocket::Websocket::new();
    async_std::task::spawn(websocket.clone().bind(websocket_socket));
    async_std::task::block_on(async move {
        let mut best_block = get_best_block(&db);
        loop {
            let (memory_changeset, storage_changeset, new_block, transactions) = new_block_receiver
                .recv()
                .race(mine_next_block(
                    &mut api_state,
                    &mut vm_state,
                    best_block.clone(),
                ))
                .await
                .unwrap();
            if is_next_block(&best_block, &new_block) {
                new_block.clone().insert(&db, transactions.clone());
                websocket
                    .send::<api::Block>((&new_block, &transactions).into())
                    .await;
                redis2.apply(memory_changeset);
                rocksdb.apply(storage_changeset);
                best_block = Some(new_block);
            } else {
                best_block = best_block.clone();
            }
        }
    });
}
