use crate::{
    config::verification_key,
    constants::{BLOCK_TIME, CURRENT_MINER_CHANNEL, TRANSACTION_QUEUE},
    helpers::run_for,
    models::{Block, Transaction},
    slasher::slash_winner,
};
use async_std::{future::timeout, task::sleep};
use futures::future::FutureExt;
use std::time::Duration;

pub async fn run() {
    loop {
        if let Ok(current_miner) = timeout(
            *BLOCK_TIME + Duration::from_secs(2),
            CURRENT_MINER_CHANNEL.1.recv().map(Result::unwrap),
        )
        .await
        {
            if current_miner.address == verification_key() {
                mine_block().await
            } else {
                sleep(*BLOCK_TIME).await;
            }
        } else {
            slash_winner().await
        }
    }
}

async fn mine_block() {
    let block = Block::insert();
    println!("Won block #{}", &block.number);
    let mut transaction_position = 0;
    run_for(*BLOCK_TIME, async {
        loop {
            let (transaction_request, sender) = TRANSACTION_QUEUE.1.recv().await.unwrap();
            let transaction =
                Transaction::run(&block, transaction_request, transaction_position as i32).await;
            transaction_position += 1;
            sender.send(transaction).unwrap();
        }
    })
    .await;
    block.seal(transaction_position + 1).await;
}
