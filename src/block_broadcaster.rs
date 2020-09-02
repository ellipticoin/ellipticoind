use crate::{
    models::{Block, Transaction},
    helpers::peers,
};

pub async fn broadcast(block: (Block, Vec<Transaction>)) {
    for peer in peers().await {
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
