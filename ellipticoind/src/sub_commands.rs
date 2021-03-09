use crate::{
    api,
    config::{socket, GENESIS_NODE},
    constants::BACKEND,
    db::{Backend, MemoryBackend},
    miner, peerchains, start_up,
};
use async_std::{sync::RwLock, task::{block_on, spawn}};
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
        let memory_backend = MemoryBackend::new();
        let backend = Backend::Memory(memory_backend);
        let db = ellipticoin_types::Db {backend: backend, transaction_state: Default::default()};
        if matches!(BACKEND.set(RwLock::new(db)), Err(_)) {
            panic!("Failed to initialize db");
        };
        let mut db2 = BACKEND.get().unwrap().write().await;
        peerchains::start(&mut db2).await
    });
    spawn(async move { listener.accept().await.unwrap() });
    miner::run().await;
}
