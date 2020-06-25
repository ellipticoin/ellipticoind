use crate::{
    api,
    models::{Block, Transaction},
    VM_STATE,
};

pub async fn broadcast(block: Block, transactions: Vec<Transaction>) {
    let mut vm_state = VM_STATE.lock().await;
    for peer in vm_state.peers().await {
        let block: api::Block = (block.clone(), transactions.clone()).into();
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
