use crate::config::my_signing_key;
use crate::helpers::bytes_to_value;
use crate::transaction::TransactionRequest;
use crate::{
    config::my_public_key,
    constants::{
        BLOCK_CHANNEL, BLOCK_SLASH_DELAY, BLOCK_TIME, MINERS, NEXT_BLOCK, TOKEN_CONTRACT,
        TRANSACTION_QUEUE,
    },
    helpers::run_for,
    models::{Block, Transaction},
    slasher::slash_winner,
    system_contracts::ellipticoin::Miner,
};
use async_std::{
    future::timeout,
    task::sleep,
    sync::{
        channel,
        Sender,
        Receiver,
    },
};
use ellipticoin::PublicKey;
use futures::future::{select_all, FutureExt};
use serde_cose::Sign1;
use std::time::Duration;

pub async fn run() {
    loop {
        let mut next_miner_index: usize = 1;
        let number: u32;
        let miner: Miner;
        {
            let next = NEXT_BLOCK.read().await.clone().unwrap();
            number = next.number;
            miner = next.miner.clone();
        }

        let (cancel_sender, cancel_receiver): (Sender<bool>, Receiver<bool>) = channel(1);
        let (received_block, _, _) = select_all(vec![
            wait_for_block(cancel_receiver).boxed(),
            wait_for_block_timeout().boxed(),
        ])
        .await;

        if received_block
            || !try_vote_no(
                number,
                miner.clone(),
                MINERS.count().await,
                MINERS.miner_at_index(next_miner_index).await.unwrap(),
            )
            .await
        {
            continue;
        }
        cancel_sender.send(true).await;
        next_miner_index += 1;



    }
}

async fn wait_for_block(cancel_channel: Receiver<bool>) -> bool {
    loop {
       match BLOCK_CHANNEL.1.try_recv() {
           Ok(mined_block_number) => {
               println!("Miner received block {}", mined_block_number);
               return true
           }
           Err(x) => {
               if cancel_channel.try_recv().is_ok() {
                   return false
               }
               sleep(Duration::from_millis(10)).await;
           }
       }
    }
    true
}

async fn wait_for_block_timeout() -> bool {
    let _ = timeout(
        *BLOCK_TIME + *BLOCK_SLASH_DELAY,
        sleep(Duration::from_secs(999999999)),
    )
    .await;

    false
}

async fn try_vote_no(
    block_number: u32,
    miner: Miner,
    miner_count: usize,
    next_miner: Miner,
) -> bool {
    let mut next_block = NEXT_BLOCK.write().await.clone().unwrap();

    if block_number != next_block.number || miner != next_block.miner {
        false
    } else {
        let signed_burn_tx: Sign1 = get_signed_burn_tx(miner.address, block_number);
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

fn get_signed_burn_tx(miner_address: PublicKey, block_number: u32) -> Sign1 {
    let burn_miner_tx = TransactionRequest::new(
        TOKEN_CONTRACT.clone(),
        "burn_winning_miner",
        vec![bytes_to_value(miner_address.to_vec()), block_number.into()],
    );
    let mut burn_tx = Sign1::new(burn_miner_tx, my_public_key().to_vec());
    burn_tx.sign(my_signing_key());
    burn_tx
}
