use crate::{
    config::verification_key,
    constants::{BLOCK_TIME, NEW_BLOCK_CHANNEL, STATE, TRANSACTION_QUEUE},
    helpers::run_for,
    models::{Block, Transaction},
    system_contracts::ellipticoin::State,
};
use async_std::{
    future::{timeout, TimeoutError},
    task::sleep,
};
use futures::future::FutureExt;
use std::time::Duration;

pub async fn run() {
    loop {
        match timeout(
            *BLOCK_TIME + Duration::from_secs(2),
            NEW_BLOCK_CHANNEL.1.recv().map(Result::unwrap),
        )
        .await
        {
            Ok(state) => mine_if_winner(state).await,
            Err(TimeoutError { .. }) => wait_for_peer().await,
        }
    }
}

async fn wait_for_peer() {
    let current_miner = STATE.current_miner().await;
    println!(
        "Waiting for peer: {} ({})",
        current_miner.host,
        base64::encode(&current_miner.address)
    );
    let state = NEW_BLOCK_CHANNEL.1.recv().map(Result::unwrap).await;
    mine_if_winner(state).await
}

async fn mine_if_winner(state: State) {
    if state
        .miners
        .first()
        .map(|miner| miner.address == verification_key())
        .unwrap_or(false)
    {
        mine_block(state.block_number).await
    } else {
        sleep(*BLOCK_TIME).await;
    }
}
async fn mine_block(block_number: u32) {
    let block = Block::insert(block_number);
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
    block.seal(transaction_position).await;
}
