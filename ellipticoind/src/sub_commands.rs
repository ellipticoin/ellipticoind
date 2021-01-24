use crate::{
    api,
    config::{socket, GENESIS_NODE},
    db::MemoryDB,
    miner, peerchains, start_up,
    state::IN_MEMORY_STATE,
};
use async_std::task::{block_on, spawn};
use ellipticoin_peerchain_ethereum::address_to_string;
use ellipticoin_peerchain_ethereum::eth_address;
use k256::ecdsa::SigningKey;
use k256::ecdsa::VerifyingKey;
use rand::rngs::OsRng;
use std::convert::TryInto;
use tide::listener::Listener;

pub fn generate_keypair() {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);
    println!(
        "Verifing Key (Address): {}",
        address_to_string(eth_address(verifying_key).try_into().unwrap())
    );
    println!("Signing Key: {}", hex::encode(signing_key.to_bytes()));
}

pub async fn main() {
    start_up::reset_state().await;
    if !*GENESIS_NODE {
        start_up::catch_up().await;
    }
    start_up::start_miner().await;
    let api = api::API::new();
    let mut listener = api.app.bind(socket()).await.unwrap();
    for info in listener.info().iter() {
        println!("Server listening on {}", info);
    }
    block_on(async {
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut db = MemoryDB::new(&mut state);
        peerchains::start(&mut db).await
    });
    spawn(async move { listener.accept().await.unwrap() });
    miner::run().await;
}
