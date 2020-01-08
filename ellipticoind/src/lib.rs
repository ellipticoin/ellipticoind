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
use async_std::sync::channel;
use async_std::sync::{Receiver, Sender};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use models::is_next_block;
use std::net::SocketAddr;
use std::sync::Arc;
use vm::rocksdb::ops::Open;
use vm::Backend;

pub async fn run(
    api_socket: SocketAddr,
    websocket_socket: SocketAddr,
    database_url: &str,
    rocksdb_path: &str,
    redis_url: &str,
    socket: SocketAddr,
    private_key: Vec<u8>,
    bootnodes: Vec<(SocketAddr, Vec<u8>)>,
    system_contract: Vec<u8>,
) {
    let (network_sender, network_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(1);
    let mut network = Server::new(private_key, socket, bootnodes).await;
    let incomming_network_receiver = network.receiver().await;
    // async_std::task::spawn(async move {
    // });
    diesel_migrations::embed_migrations!();
    let db = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));
    embedded_migrations::run(&db).unwrap();
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    let redis = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let mut redis2 = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let mut rocksdb = Arc::new(vm::rocksdb::DB::open_default(rocksdb_path).unwrap());
    let mut api_state = api::State::new(redis, rocksdb.clone(), pg_pool, network_sender);
    let mut vm_state = vm::State::new(redis2.get_connection().unwrap(), rocksdb.clone());
    vm_state.set_code(&TOKEN_CONTRACT.to_vec(), &system_contract.to_vec());
    let (new_block_sender, mut new_block_receiver) = channel(1);
    diesel::sql_query("TRUNCATE blocks CASCADE")
        .execute(&db)
        .unwrap();
    async_std::task::spawn(api(api_state.clone()).listen(api_socket));
    async_std::task::spawn(network::handle_messages(
        api_state.clone(),
        incomming_network_receiver,
        new_block_sender,
    ));
    let mut websocket = api::websocket::Websocket::new();
    async_std::task::spawn(websocket.clone().bind(websocket_socket));
    async_std::task::spawn(async move {
        let mut best_block = get_best_block(&db);
        loop {
            use futures::{future::FutureExt, pin_mut, select};
            use futures::stream::StreamExt;
            let network_receiver_fused = new_block_receiver.next().map(Option::unwrap).fuse();
            let mine_next_block_fused =
                mine_next_block(&mut api_state, &mut vm_state, best_block.clone()).fuse();
            pin_mut!(network_receiver_fused, mine_next_block_fused);
            let (mined, (memory_changeset, storage_changeset, new_block, transactions)) = select! {
                result = mine_next_block_fused => (true, result),
                result = network_receiver_fused => (false, result),
                complete => break,
            };
            if is_next_block(&best_block, &new_block) {
                println!("Inserting block hash {}", base64::encode(&new_block.hash)[0..4])
                new_block.clone().insert(&db, transactions.clone());
                websocket
                    .send::<api::Block>((&new_block, &transactions).into())
                    .await;
                redis2.apply(memory_changeset);
                rocksdb.apply(storage_changeset);
                if mined {
                    println!("Mined block #{}", &new_block.number);
                } else {
                    println!("Applied block #{}", &new_block.number);
                }
                best_block = Some(new_block);
            } else {
                best_block = best_block.clone();
            }
        };
    });

    network.listen(network_receiver).await;
}
