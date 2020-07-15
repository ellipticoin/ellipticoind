use crate::{
    api::Message, block_broadcaster::broadcast, config::public_key, constants::BLOCK_TIME,
    helpers::current_miner, models, models::Block, state::State,
};
use async_std::{sync, task::sleep};
use ellipticoin::Address;
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};

use broadcaster::BroadcastChannel;

pub async fn run(
    mut state: State,
    new_block_sender: BroadcastChannel<Vec<u8>>,
    mut api_receiver: sync::Receiver<Message>,
) {
    'run: loop {
        if current_miner()
            .address
            .eq(&Address::PublicKey(public_key()))
        {
            let block = Block::insert(&mut state).await;
            println!("Won block #{}", &block.number);
            let sleep_fused = sleep(*BLOCK_TIME).fuse();
            pin_mut!(sleep_fused);
            loop {
                let mut transaction_position = 0;
                let next_message_fused = api_receiver.next().map(Option::unwrap).fuse();
                pin_mut!(next_message_fused);
                select! {
                    () = sleep_fused => {
                        let transactions = block.seal(&mut state, transaction_position + 1).await;
                        broadcast(&mut state, (block.clone(), transactions.clone())).await;
                        new_block_sender.send(&block.hash).await.unwrap();
                        continue 'run;
                    },
                    (message) = next_message_fused => {
                        match message {
                            Message::Block(block) => {
                                println!("Got block while mining");
                            },
                            Message::Transaction(transaction, responder) => {
                                let completed_transaction =
                                    models::Transaction::run(&mut state, &block, transaction, transaction_position);
                                transaction_position += 1;
                                responder.send(completed_transaction).unwrap();
                            },
                        }
                    },
                }
            }
        }
        if let Message::Block((block, transactions)) = api_receiver.next().map(Option::unwrap).await
        {
            block.clone().apply(&mut state, transactions.clone());
            new_block_sender.send(&block.hash).await.unwrap();
        }
    }
}
