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
mod run_loop;
pub mod schema;
mod system_contracts;
mod transaction_processor;

use crate::constants::TOKEN_CONTRACT;
use crate::miner::get_best_block;
use crate::models::Block;
use ::network::Server;
use api::app::app as api;
use async_std::sync::channel;
use async_std::sync::Mutex;
use async_std::sync::{Receiver, Sender};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use ed25519_dalek::{ExpandedSecretKey, PublicKey, SecretKey};
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::sync::Arc;
use vm::rocksdb::ops::Open;
lazy_static! {
    static ref BEST_BLOCK: async_std::sync::Arc<Mutex<Option<Block>>> =
        { async_std::sync::Arc::new(Mutex::new(None)) };
}

pub fn generate_keypair() {
    let mut csprng = OsRng {};
    let secret_key = SecretKey::generate(&mut csprng);
    let expanded_secret_key: ExpandedSecretKey = (&secret_key).into();
    let public_key: PublicKey = (&secret_key).into();
    println!("Public Key (Address): {}", base64::encode(&public_key));
    println!(
        "Private Key: {}",
        base64::encode(&expanded_secret_key.to_bytes().to_vec())
    );
}

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
    diesel_migrations::embed_migrations!();
    let (network_sender, network_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(1);
    let mut network = Server::new(private_key, socket, bootnodes).await;
    let incomming_network_receiver = network.receiver().await;
    let db = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));
    embedded_migrations::run(&db).unwrap();
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    let redis = vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    let redis2 = vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    let rocksdb = Arc::new(vm::rocksdb::DB::open_default(rocksdb_path).unwrap());
    let api_state = api::State::new(redis, rocksdb.clone(), pg_pool, network_sender);
    let api_state2 = api_state.clone();
    let mut vm_state = vm::State::new(redis2.get_connection().unwrap(), rocksdb.clone());
    vm_state.set_code(&TOKEN_CONTRACT.to_vec(), &system_contract.to_vec());
    let (new_block_sender, new_block_receiver) = channel(1);
    diesel::sql_query("TRUNCATE blocks CASCADE")
        .execute(&db)
        .unwrap();
    async_std::task::spawn(api(api_state.clone()).listen(api_socket));
    async_std::task::spawn(network::handle_messages(
        api_state.clone(),
        incomming_network_receiver,
        new_block_sender,
    ));
    let websocket = api::websocket::Websocket::new();
    async_std::task::spawn(websocket.clone().bind(websocket_socket));
    *BEST_BLOCK.lock().await = get_best_block(&db);
    async_std::task::spawn(async {
        run_loop::run(websocket, redis2, rocksdb, api_state2, new_block_receiver).await
    });
    network.listen(network_receiver).await;
}
