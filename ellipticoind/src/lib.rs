#![recursion_limit = "512"]
extern crate bytes;
extern crate hex;
extern crate mime;
extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate sha2;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate hex_literal;

mod api;
mod broadcaster;
pub mod config;
mod constants;
mod helpers;
mod miner;
pub mod models;
mod run_loop;
pub mod schema;
mod start_up;
mod system_contracts;
mod transaction_processor;

use crate::miner::get_best_block;
use crate::models::Block;
use api::app::app as api;
use async_std::sync::channel;
use async_std::sync::Mutex;
use async_std::task::sleep;
use broadcaster::broadcast;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use ed25519_dalek::Keypair;
pub use futures::{sink::SinkExt, stream::StreamExt};
use rand::rngs::OsRng;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use vm::{r2d2, redis, RedisConnectionManager};
// use r2d2_redis::{r2d2, RedisConnectionManager};
use vm::redis::Commands;

lazy_static! {
    static ref BEST_BLOCK: async_std::sync::Arc<Mutex<Option<Block>>> =
        async_std::sync::Arc::new(Mutex::new(None));
}

pub fn generate_keypair() {
    let mut csprng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut csprng);
    println!(
        "Public Key (Address): {}",
        base64::encode(&keypair.public.to_bytes())
    );
    println!(
        "Private Key: {}",
        base64::encode(&keypair.to_bytes().to_vec())
    );
}

pub async fn run(
    database_url: String,
    rocksdb_path: &str,
    redis_url: &str,
    socket: SocketAddr,
    websocket_port: u16,
    keypair: Keypair,
    bootnodes: Vec<crate::config::Bootnode>,
) {
    diesel_migrations::embed_migrations!();
    let db = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));
    embedded_migrations::run(&db).unwrap();
    let manager = ConnectionManager::<PgConnection>::new(database_url.clone());
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    let redis_manager = RedisConnectionManager::new(redis_url).unwrap();
    let redis_pool = vm::r2d2_redis::r2d2::Pool::builder()
        .build(redis_manager)
        .unwrap();

    let _: () = redis::cmd("FLUSHALL")
        .query(&mut *redis_pool.get().unwrap())
        .unwrap();

    let rocksdb = Arc::new(
        start_up::initialize_rocks_db(rocksdb_path, &pg_pool.get().unwrap(), redis_pool.clone())
            .await,
    );
    let mut vm_state = vm::State::new(redis_pool.get().unwrap(), rocksdb.clone());
    if env::var("GENISIS_NODE").is_err() {
        crate::start_up::catch_up(
            &pg_pool.get().unwrap(),
            redis_pool.clone(),
            &mut vm_state,
            &bootnodes,
        )
        .await;
    }
    use std::io::Read;
    let mut token_file = std::fs::File::open("../token/dist/token.wasm").unwrap();
    let mut token_wasm = Vec::new();
    token_file.read_to_end(&mut token_wasm).unwrap();
    rocksdb
        .put(
            vm::state::db_key(&crate::constants::TOKEN_CONTRACT, &vec![]),
            &token_wasm,
        )
        .unwrap();
    start_up::start_miner(
        &rocksdb,
        &pg_pool.get().unwrap(),
        redis_pool.clone(),
        keypair.public,
    )
    .await;
    let api_state = api::State::new(redis_pool.clone(), rocksdb.clone(), pg_pool);
    let (block_sender_in, block_receiver_in) = channel(1);
    let (block_sender_out, block_receiver_out) = channel(1);
    async_std::task::spawn(api(api_state).listen(socket));
    async_std::task::spawn(broadcast(block_receiver_out, vm_state));
    let websocket = api::websocket::Websocket::new();
    let mut websocket_socket = socket.clone();
    websocket_socket.set_port(websocket_port);
    async_std::task::spawn(websocket.clone().bind(websocket_socket));
    *BEST_BLOCK.lock().await = get_best_block(&db);
    // let public = private_key.public.to_bytes();
    let public_key = Arc::new(keypair.public);
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    run_loop::run(
        public_key,
        websocket,
        redis_pool.clone(),
        rocksdb,
        pg_pool,
        block_receiver_in,
        block_sender_out,
    )
    .await
}
