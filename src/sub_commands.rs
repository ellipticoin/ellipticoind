use crate::{
    api,
    config::{
         socket, websocket_socket, ENABLE_MINER, GENESIS_NODE,
    },
    run_loop, start_up,
    VM_STATE, WEB_SOCKET,
};
use async_std::task::spawn;
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
    let mut vm_state = VM_STATE.lock().await;
    start_up::reset_state().await;
    if !*GENESIS_NODE {
        start_up::catch_up(&mut vm_state).await;
    }
    if *ENABLE_MINER {
        start_up::start_miner(&mut vm_state).await;
    }
    let (api_receiver, api_state) = api::API::new();
    spawn(api_state.listen(socket()));
    spawn(
        (*WEB_SOCKET)
            .lock()
            .await
            .clone()
            .bind(websocket_socket().await),
    );
    run_loop::run(&mut vm_state, api_receiver).await
}
