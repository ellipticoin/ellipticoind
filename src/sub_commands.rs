use crate::{
    api,
    broadcaster::broadcast,
    miner::get_best_block,
    run_loop, start_up, vm,
    vm::{rocksdb, RedisConnectionManager},
};
use async_std::sync::channel;
use diesel::{
    pg::PgConnection,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

use crate::{
    config::{bootnodes, database_url, keypair, socket, GENESIS_NODE, OPTS},
    BEST_BLOCK,
};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use std::sync::Arc;

pub fn generate_keypair() {
    let mut os_rng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut os_rng);
    let public_key = base64::encode(&keypair.public.to_bytes());
    let private_key = base64::encode(&keypair.to_bytes().to_vec());
    println!("Public Key (Address): {}", public_key);
    println!("Private Key: {}", private_key);
}

pub async fn main() {
    let db = PgConnection::establish(&database_url())
        .expect(&format!("Error connecting to {}", &database_url()));
    let manager = ConnectionManager::<PgConnection>::new(&database_url());
    let pg_pool = Pool::new(manager).unwrap();
    let redis_manager = RedisConnectionManager::new(OPTS.redis_url.clone()).unwrap();
    let redis_pool = vm::r2d2_redis::r2d2::Pool::builder()
        .build(redis_manager)
        .unwrap();

    let rocksdb = Arc::new(rocksdb::DB::open_default(&OPTS.rocksdb_path).unwrap());

    start_up::reset_state(rocksdb.clone(), &pg_pool.get().unwrap(), redis_pool.clone()).await;
    let mut vm_state = vm::State::new(redis_pool.get().unwrap(), rocksdb.clone());

    if !*GENESIS_NODE {
        start_up::catch_up(
            pg_pool.clone(),
            redis_pool.clone(),
            &mut vm_state,
            &bootnodes(),
        )
        .await;
        start_up::start_miner(
            &rocksdb,
            &pg_pool.get().unwrap(),
            keypair().public,
            &bootnodes(),
        )
        .await;
    }
    let (miner_sender, miner_receiver) = channel(1);
    let (broadcast_sender, broadcast_receiver) = channel(1);
    let api_state = api::ApiState::new(
        redis_pool.clone(),
        rocksdb.clone(),
        pg_pool,
        broadcast_sender.clone(),
        miner_sender,
    );
    async_std::task::spawn(api(api_state).listen(socket()));
    async_std::task::spawn(broadcast(
        broadcast_receiver,
        redis_pool.clone(),
        rocksdb.clone(),
    ));
    let websocket = api::websocket::Websocket::new();
    let mut websocket_socket = socket().clone();
    websocket_socket.set_port(OPTS.websocket_port);
    async_std::task::spawn(websocket.clone().bind(websocket_socket));
    *BEST_BLOCK.lock().await = get_best_block(&db);
    let manager = ConnectionManager::new(&database_url());
    let pg_pool = Pool::new(manager).unwrap();
    run_loop::run(
        websocket,
        redis_pool.clone(),
        rocksdb,
        pg_pool,
        miner_receiver,
        broadcast_sender,
    )
    .await
}
