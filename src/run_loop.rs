use crate::{
    api, block_broadcaster::broadcast, constants::BLOCK_TIME, models::Block, network::Message,
    vm::State, VM_STATE, WEB_SOCKET,
};
use async_std::{sync, task::sleep};
use futures::{future::FutureExt, stream::StreamExt};

pub async fn run(mut receiver: sync::Receiver<Message>) {
    loop {
        if State::is_block_winner().await {
            let block = Block::insert().await;
            {
                let mut vm_state = VM_STATE.lock().await;
                println!("Won block #{}", vm_state.block_number());
            }
            sleep(*BLOCK_TIME).await;
            let transactions = block.seal().await;
            broadcast(block.clone(), transactions.clone()).await;
            (*WEB_SOCKET)
                .lock()
                .await
                .send::<api::Block>((block, transactions).into())
                .await;
            continue;
        }
        if let Message::Block((block, transactions)) = receiver.next().map(Option::unwrap).await {
            if block.is_valid().await {
                block.clone().apply(transactions.clone()).await;
                (*WEB_SOCKET)
                    .lock()
                    .await
                    .send::<api::Block>((block, transactions).into())
                    .await;
            }
        }
    }
}
