use crate::{
    config::my_public_key,
    constants::{BLOCK_TIME, BLOCK_SLASH_DELAY, BLOCK_CHANNEL, MINERS, NEXT_BLOCK, TRANSACTION_QUEUE},
    helpers::run_for,
    models::{Block, Transaction},
    slasher::slash_winner,
    system_contracts::ellipticoin::Miner,
};
use async_std::{future::timeout, task::sleep};
use futures::future::{
    FutureExt,
    select_all
};
use std::time::Duration;
use std::process;
use serde_cose::Sign1;
use crate::config::my_signing_key;

pub async fn run() {
    // TODO: Swap out for a channel in separate function
    for attempt in 1i32..7i32 {
        if let Some(_) = *NEXT_BLOCK.read().await {
            break;
        }
        sleep(Duration::from_secs(1)).await;
        if attempt > 5 {
            eprintln!("Error starting. Sync did not start!");
            process::exit(1);
        }
    }

    loop {
        let number: i32;
        let miner: Miner;
        {
            let next = NEXT_BLOCK.read().await.clone().unwrap();
            number = next.number;
            miner = next.miner.clone();
        }

        let (received_block, _, _) = select_all(vec![wait_for_block().boxed(), wait_for_block_timeout().boxed()]).await;

        if received_block || !try_vote_no(number, miner.clone(), MINERS.count().await, MINERS.second().await).await {
            continue;
        }

        // TODO: Wait for everybody to burn this guy or vote for it.
    }
}

async fn wait_for_block() -> bool {
    BLOCK_CHANNEL.1.recv().map(Result::unwrap);

    true
}

async fn wait_for_block_timeout() -> bool {
    let _ = timeout(
        *BLOCK_TIME + *BLOCK_SLASH_DELAY,
        sleep(Duration::from_secs(999999999))
    ).await;

    false
}

async fn try_vote_no(block_number: i32, miner: Miner, miner_count: usize, next_miner: Miner) -> bool {
    let mut next_block = NEXT_BLOCK.write().await.clone().unwrap();

    if block_number != next_block.number || miner != next_block.miner {
        false
    } else {
        let signed_burn_tx: Sign1 = get_signed_burn_tx(&miner);
        next_block.burn_current_miner(&signed_burn_tx, miner_count, &next_miner);

        // TODO: Send burn notice to all other miners

        true
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

fn get_signed_burn_tx(miner: &Miner) -> Sign1 {
    // TODO: actually get burn tx.
    let mut burn_tx = Sign1::new("derp", my_public_key().to_vec());
    burn_tx.sign(my_signing_key());
    burn_tx
}
