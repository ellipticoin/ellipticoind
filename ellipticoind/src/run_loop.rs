use crate::api;
use crate::miner::mine_next_block;
use crate::models::is_next_block;
use crate::models::{Block, Transaction};
use crate::transaction_processor;
use crate::BEST_BLOCK;
use async_std::sync::Receiver;
use futures::stream::StreamExt;
use futures::{future::FutureExt, pin_mut, select};
use vm::Backend;
pub async fn run(
    mut websocket: api::websocket::Websocket,
    mut redis: redis::Client,
    mut rocksdb: std::sync::Arc<rocksdb::DB>,
    mut api_state: api::State,
    mut new_block_receiver: Receiver<(Block, Vec<Transaction>)>,
) {
    let api_state2 = api_state.clone();
    let db = api_state.db.get().unwrap();
    let mut vm_state = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
    loop {
        let network_receiver_fused = new_block_receiver.next().map(Option::unwrap).fuse();
        let mine_next_block_fused = mine_next_block(&mut api_state, &mut vm_state).fuse();
        pin_mut!(network_receiver_fused, mine_next_block_fused);
        select! {
            (new_block, transactions) = mine_next_block_fused => {
                if is_next_block(&new_block).await {
                    new_block.clone().insert(&db, transactions.clone());
                    websocket
                        .send::<api::Block>((&new_block, &transactions).into())
                        .await;
                    println!("Mined block #{}", &new_block.number);
                    *BEST_BLOCK.lock().await = Some(new_block.clone());
                }
            },
            (new_block, transactions) = network_receiver_fused => {
                if is_next_block(&new_block).await {
                    new_block.clone().insert(&db, transactions.clone());
                    let mut vm_state2 = api_state2.vm_state();
                    transaction_processor::apply_block(
                        &mut vm_state2,
                        new_block.clone(),
                        transactions.clone(),
                        );
                    redis.apply(vm_state2.memory_changeset.clone());
                    rocksdb.apply(vm_state2.storage_changeset.clone());
                    websocket
                        .send::<api::Block>((&new_block, &transactions).into())
                        .await;
                    println!("Applied block #{}", &new_block.number);
                    *BEST_BLOCK.lock().await = Some(new_block.clone());
                }
            },
            complete => break,
        }
    }
}
