use crate::{
    client::post_block,
    config::HOST,
    constants::BLOCK_BROADCASTER,
    models::{block::Block, transaction::Transaction},
    system_contracts::ellipticoin::Miner,
};
use futures::future::{BoxFuture, FutureExt};

pub fn broadcast_block(
    block: (Block, Vec<Transaction>),
    miners: Vec<Miner>,
) -> BoxFuture<'static, ()> {
    async move {
        for miner in miners
            .iter()
            .cloned()
            .filter(|miner| miner.host.to_string() != *HOST)
            .collect::<Vec<Miner>>()
        {
            post_block(miner.host, &block).await
        }
        BLOCK_BROADCASTER
            .send(&(block.0.number as u32))
            .await
            .unwrap();
    }
    .boxed()
}
