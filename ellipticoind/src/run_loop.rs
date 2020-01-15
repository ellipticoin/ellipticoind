use crate::api;
use crate::miner::mine_next_block;
use crate::models::is_next_block;
use crate::models::{Block, Transaction};
use crate::network::Message;
use crate::transaction_processor;
use crate::BEST_BLOCK;
use async_std::sync;
use diesel::r2d2::{ConnectionManager, Pool};
use futures::stream::StreamExt;
use futures::{future::FutureExt, pin_mut, select};
use network::Sender;
pub async fn run(
    mut websocket: api::websocket::Websocket,
    mut network_sender: Sender,
    redis: redis::Client,
    rocksdb: std::sync::Arc<rocksdb::DB>,
    db_pool: Pool<ConnectionManager<diesel::PgConnection>>,
    mut new_block_receiver: sync::Receiver<(Block, Vec<Transaction>)>,
) {
    let db = db_pool.get().unwrap();
    loop {
        let vm_state = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
        let mut vm_state2 = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
        let network_receiver_fused = new_block_receiver.next().map(Option::unwrap).fuse();
        let mut redis_connection = redis.get_connection().unwrap();
        let mine_next_block_fused = mine_next_block(&mut redis_connection, vm_state).fuse();
        pin_mut!(network_receiver_fused, mine_next_block_fused);
        select! {
            ((new_block, transactions), mut vm_state) = mine_next_block_fused => {
                if is_next_block(&new_block).await {
                    vm_state.commit();
                    new_block.clone().insert(&db, transactions.clone());
                    websocket
                        .send::<api::Block>((&new_block, &transactions).into())
                    .await;
                    network_sender
                        .send(&Message::Block((new_block.clone(), transactions.clone())))
                    .await;
                    println!("Mined block #{}", &new_block.number);
                    *BEST_BLOCK.lock().await = Some(new_block.clone());
                }
            },
            (new_block, transactions) = network_receiver_fused => {
                if is_next_block(&new_block).await {
                    new_block.clone().insert(&db, transactions.clone());
                    transaction_processor::apply_block(
                        &mut vm_state2,
                        new_block.clone(),
                        transactions.clone(),
                        );
                    vm_state2.commit();
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
