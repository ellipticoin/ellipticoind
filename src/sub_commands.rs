use crate::{
    api,
    config::{socket, websocket_socket, ENABLE_MINER, GENESIS_NODE},
    run_loop, start_up, WEB_SOCKET,
};
use async_std::{sync::channel, task::spawn};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;

pub fn generate_keypair() {
    let mut os_rng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut os_rng);
    let public_key = base64::encode(&keypair.public.to_bytes());
    let private_key = base64::encode(&keypair.to_bytes().to_vec());
    println!("Public Key (Address): {}", public_key);
    println!("Private Key: {}", private_key);
}

pub async fn main() {
    start_up::reset_state().await;
    if !*GENESIS_NODE {
        start_up::catch_up().await;
    }
    if *ENABLE_MINER {
        start_up::start_miner().await;
    }
    let (miner_sender, miner_receiver) = channel(1);
    let api_state = api::ApiState::new(miner_sender);
    spawn(api(api_state).listen(socket()));
    spawn(
        (*WEB_SOCKET)
            .lock()
            .await
            .clone()
            .bind(websocket_socket().await),
    );
    run_loop::run(miner_receiver).await
}
