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

mod api;
mod helpers;
mod miner;
mod constants;
pub mod models;
pub mod schema;
mod system_contracts;

use api::API;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::net::SocketAddr;
use vm::rocksdb::ops::Open;
use diesel::r2d2::{ConnectionManager, Pool};

pub const ROCKSDB_PATH: &str = "./db";

pub async fn run(
    socket: SocketAddr,
    database_url: &str,
    redis_url: &str,
    system_contract: Vec<u8>,
) {
    let db = PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url));
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pg_pool = Pool::new(manager).expect("Postgres connection pool could not be created");

    let redis = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let redis2 = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let api = API::new(redis, pg_pool);
    let mut api2 = api.clone();
    let rocksdb = vm::rocksdb::DB::open_default(ROCKSDB_PATH).unwrap();
    let mut vm_state = vm::State::new(
        redis2.get_connection().unwrap(),
        rocksdb,
        system_contract.to_vec(),
    );
    tokio::spawn(async move {
        miner::mine(db, &mut api2, &mut vm_state).await;
    });
    api.serve(socket).await;
}
