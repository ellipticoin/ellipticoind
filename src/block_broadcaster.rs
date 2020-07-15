use crate::{
    models::{Block, Transaction},
    state::State,
};

pub async fn broadcast(vm_state: &mut State, block: (Block, Vec<Transaction>)) {
    for peer in vm_state.peers().await {
        let uri = format!("http://{}/blocks", peer);
        if surf::post(uri)
            .body_bytes(serde_cbor::to_vec(&block).unwrap())
            .await
            .is_err()
        {
            println!("failed posting to http://{}/blocks", peer);
        }
    }
}
