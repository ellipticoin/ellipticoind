use crate::{
    client::post_block,
    helpers::peers,
    models::{Block, Transaction},
};

pub async fn broadcast(block: (Block, Vec<Transaction>)) {
    for peer in peers().await {
        post_block(peer, &block).await;
    }
}
