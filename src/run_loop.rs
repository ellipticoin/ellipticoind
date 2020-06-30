use crate::{
    api, api::Message, block_broadcaster::broadcast, config::public_key, constants::BLOCK_TIME,
    models, models::Block, vm::State, WEB_SOCKET,
};
use async_std::{sync, task::sleep};
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};

pub async fn run(mut vm_state: &mut State, mut api_receiver: sync::Receiver<Message>) {
    'run: loop {
        let block;
        if vm_state.current_miner().map_or(false, |current_miner| {
            current_miner.address.eq(&public_key())
        }) {
            block = Block::insert(&mut vm_state).await;
            println!("Won block #{}", &block.number);
            let sleep_fused = sleep(*BLOCK_TIME).fuse();
            pin_mut!(sleep_fused);
            loop {
                let mut transaction_position = 0;
                let next_message_fused = api_receiver.next().map(Option::unwrap).fuse();
                pin_mut!(next_message_fused);
                select! {
                    () = sleep_fused => {
                        let transactions = block.seal(&mut vm_state, transaction_position).await;
                        broadcast(&mut vm_state, (block.clone(), transactions.clone())).await;
                        (*WEB_SOCKET)
                            .lock()
                            .await
                            .send::<api::views::Block>((block.clone(), transactions).into())
                            .await;
                        continue 'run;
                    },
                    (message) = next_message_fused => {
                        match message {
                            Message::Block(block) => {
                                println!("Got block while mining");
                            },
                            Message::Transaction(transaction, responder) => {
                                let completed_transaction =
                                    models::Transaction::run(&mut vm_state, &block, transaction, transaction_position);
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
            block.clone().apply(&mut vm_state, transactions.clone());
            (*WEB_SOCKET)
                .lock()
                .await
                .send::<api::views::Block>((block, transactions).into())
                .await;
        }
    }
}
