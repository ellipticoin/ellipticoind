use crate::{
    client::post_block,
    config::HOST,
    constants::BLOCK_BROADCASTER,
    models::{block::Block, transaction::Transaction},
    system_contracts::ellipticoin::Miner,
};
use futures::future::{join_all, BoxFuture, FutureExt};

pub fn broadcast_block(
    block: (Block, Vec<Transaction>),
    miners: Vec<Miner>,
) -> BoxFuture<'static, ()> {
    async move {
        join_all(
            miners
                .iter()
                .cloned()
                .filter(|miner| miner.host.to_string() != *HOST)
                .map(|miner| post_block(miner.host, &block)),
        )
        .await;

        BLOCK_BROADCASTER
            .send(&(block.0.number as u32))
            .await
            .unwrap();
    }
    .boxed()
}
