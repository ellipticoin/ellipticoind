#![feature(async_closure)]
extern crate bytes;
extern crate hex;
extern crate mime;
extern crate rocksdb;
extern crate serde_cbor;
extern crate tokio;

mod api;
mod miner;
mod system_contracts;

use api::API;
use std::net::SocketAddr;
use vm::rocksdb::ops::Open;

pub const ROCKSDB_PATH: &str = "./db";

pub async fn run(socket: SocketAddr, redis_url: &str, system_contract: Vec<u8>) {
    let redis = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let redis2 = vm::redis::Client::open::<&str>(redis_url.into()).unwrap();
    let api = API::new(redis);
    let mut api2 = api.clone();
    let rocksdb = vm::rocksdb::DB::open_default(ROCKSDB_PATH).unwrap();
    let mut vm_state = vm::State::new(
        redis2.get_connection().unwrap(),
        rocksdb,
        system_contract.to_vec(),
    );
    tokio::spawn(async move {
        miner::mine(&mut api2, &mut vm_state).await;
    });
    api.serve(socket).await;
}
