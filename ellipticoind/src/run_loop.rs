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
        let mut vm_state = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
        let (new_block, transactions) = new_block_receiver.next().map(Option::unwrap).await;
        if is_next_block(&new_block).await {
            new_block.clone().insert(&db, transactions.clone());
            transaction_processor::apply_block(
                &mut vm_state,
                new_block.clone(),
                transactions.clone(),
            );
            vm_state.commit();
            websocket
                .send::<api::Block>((&new_block, &transactions).into())
                .await;
            println!("Applied block #{}", &new_block.number);
            *BEST_BLOCK.lock().await = Some(new_block.clone());
        }
    }
}
