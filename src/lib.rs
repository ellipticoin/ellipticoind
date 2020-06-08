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

mod api;
mod broadcaster;
pub mod config;
mod constants;
mod helpers;
mod miner;
mod models;
mod network;
mod run_loop;
mod schema;
mod start_up;
mod system_contracts;
mod transaction_processor;
mod vm;

use crate::config::Bootnode;
use crate::miner::get_best_block;
use crate::models::Block;

use api::app::app as api;
use async_std::sync::channel;
use async_std::sync::Mutex;
use broadcaster::broadcast;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use std::{env, net::SocketAddr, sync::Arc};
use crate::vm::{redis, RedisConnectionManager};

lazy_static! {
    static ref BEST_BLOCK: async_std::sync::Arc<Mutex<Option<Block>>> =
        async_std::sync::Arc::new(Mutex::new(None));
}

pub fn generate_keypair() {
    let mut os_rng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut os_rng);
    let public_key = base64::encode(&keypair.public.to_bytes());
    let private_key = base64::encode(&keypair.to_bytes().to_vec());
    println!("Public Key (Address): {}", public_key);
    println!("Private Key: {}", private_key);
}

pub async fn run(
    database_url: String,
    rocksdb_path: &str,
    redis_url: &str,
    socket: SocketAddr,
    websocket_port: u16,
    keypair: Keypair,
    bootnodes: Vec<Bootnode>,
) {
    diesel_migrations::embed_migrations!();
    let db = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));
    embedded_migrations::run(&db).unwrap();
    let manager = ConnectionManager::<PgConnection>::new(database_url.clone());
    let pg_pool = Pool::new(manager).unwrap();
    diesel::sql_query("TRUNCATE blocks CASCADE")
        .execute(&db)
        .unwrap();
    let redis_manager = RedisConnectionManager::new(redis_url).unwrap();
    let redis_pool = crate::vm::r2d2_redis::r2d2::Pool::builder()
        .build(redis_manager)
        .unwrap();
    let _: () = redis::cmd("FLUSHALL")
        .query(&mut *redis_pool.get().unwrap())
        .unwrap();

    let rocksdb = Arc::new(
        start_up::initialize_rocks_db(rocksdb_path, &pg_pool.get().unwrap(), redis_pool.clone())
            .await,
    );
    let mut vm_state = crate::vm::State::new(redis_pool.get().unwrap(), rocksdb.clone());
    if env::var("GENISIS_NODE").is_err() {
        start_up::catch_up(
            pg_pool.clone(),
            redis_pool.clone(),
            &mut vm_state,
            &bootnodes,
        )
        .await;
    }
    start_up::start_miner(
        &rocksdb,
        &pg_pool.get().unwrap(),
        redis_pool.clone(),
        keypair.public,
        &bootnodes,
    )
    .await;
    let (sender_in, receiver_in) = channel(1);
    let (sender_out, receiver_out) = channel(1);
    let api_state = api::State::new(redis_pool.clone(), rocksdb.clone(), pg_pool, sender_in);
    async_std::task::spawn(api(api_state).listen(socket));
    async_std::task::spawn(broadcast(receiver_out, vm_state));
    let websocket = api::websocket::Websocket::new();
    let mut websocket_socket = socket.clone();
    websocket_socket.set_port(websocket_port);
    async_std::task::spawn(websocket.clone().bind(websocket_socket));
    *BEST_BLOCK.lock().await = get_best_block(&db);
    let public_key = Arc::new(keypair.public);
    let manager = ConnectionManager::new(database_url);
    let pg_pool = Pool::new(manager).unwrap();
    run_loop::run(
        public_key,
        websocket,
        redis_pool.clone(),
        rocksdb,
        pg_pool,
        receiver_in,
        sender_out,
    )
    .await
}
