#![recursion_limit = "512"]
#![feature(async_closure)]
extern crate bytes;
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
#[macro_use]
extern crate hex_literal;

mod api;
mod constants;
mod helpers;
mod miner;
pub mod models;
mod network;
mod run_loop;
pub mod schema;
mod start_up;
mod system_contracts;
mod transaction_processor;

use crate::miner::get_best_block;
use crate::models::Block;
use ::network::Server;
use api::app::app as api;
use async_std::sync::channel;
use async_std::sync::Mutex;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use ed25519_dalek::Keypair;
pub use futures::{sink::SinkExt, stream::StreamExt};
use rand::rngs::OsRng;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
lazy_static! {
    static ref BEST_BLOCK: async_std::sync::Arc<Mutex<Option<Block>>> =
         async_std::sync::Arc::new(Mutex::new(None)) ;
}

pub fn generate_keypair() {
    let mut csprng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut csprng);
    println!("Public Key (Address): {}", base64::encode(&keypair.public.to_bytes()));
    println!(
        "Private Key: {}",
        base64::encode(&keypair.to_bytes().to_vec())
    );
}

pub async fn run(
    api_socket: SocketAddr,
    websocket_socket: SocketAddr,
    database_url: String,
    rocksdb_path: &str,
    redis_url: &str,
    socket: SocketAddr,
    external_socket: SocketAddr,
    keypair: Keypair,
    bootnodes: Vec<SocketAddr>,
) {
    diesel_migrations::embed_migrations!();
    let network = Server::new(keypair.to_bytes().to_vec(), socket, external_socket, bootnodes.clone());
    let (network_sender, incomming_network_receiver) = network.channel().await;
    let db = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));
    embedded_migrations::run(&db).unwrap();
    let manager = ConnectionManager::<PgConnection>::new(database_url.clone());
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    let redis =
        vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    let redis2 =
        vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    let mut redis3 =
        vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    let redis4 =
        vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    let mut redis5 =
        vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    let mut redis6 =
        vm::redis::Client::open::<&str>(redis_url.into()).expect("Failed to connect to Redis");
    diesel::sql_query("TRUNCATE blocks CASCADE")
        .execute(&db)
        .unwrap();
    let _: () = redis::cmd("FLUSHALL").query(&mut redis3).unwrap();
    let rocksdb = Arc::new(
        start_up::initialize_rocks_db(rocksdb_path, &pg_pool.get().unwrap(), &mut redis5).await,
    );
    start_up::start_miner(
        &rocksdb,
        &pg_pool.get().unwrap(),
        &mut redis6,
        keypair.public,
        network_sender.clone(),
    );
    let api_state = api::State::new(redis, rocksdb.clone(), pg_pool, network_sender.clone());
    let mut vm_state = vm::State::new(redis2.get_connection().unwrap(), rocksdb.clone());
    let (new_block_sender, new_block_receiver) = channel(1);
    async_std::task::spawn(api(api_state).listen(api_socket));
    async_std::task::spawn(network::handle_messages(
        redis4.get_connection().unwrap(),
        incomming_network_receiver,
        new_block_sender,
    ));
    let websocket = api::websocket::Websocket::new();
    async_std::task::spawn(websocket.clone().bind(websocket_socket));
    *BEST_BLOCK.lock().await = get_best_block(&db);
    // let public = private_key.public.to_bytes();
    let public_key = Arc::new(keypair.public);
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    if env::var("GENISIS_NODE").is_err() {
        crate::start_up::catch_up(&mut vm_state, &bootnodes).await;
    }
    run_loop::run(
        public_key,
        websocket,
        network_sender,
        redis2,
        rocksdb,
        pg_pool,
        new_block_receiver,
    )
    .await
}
