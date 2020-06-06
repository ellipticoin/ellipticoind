use crate::api;
use crate::miner::mine_next_block;
use crate::models::{is_block_winner, is_next_block};
use crate::models::{Block, Transaction};
use crate::transaction_processor;
use crate::BEST_BLOCK;
use async_std::sync;

use ed25519_dalek::PublicKey;

use futures::future::FutureExt;
use futures::stream::StreamExt;

pub async fn run(
    public_key: std::sync::Arc<PublicKey>,
    mut websocket: api::websocket::Websocket,
    redis: vm::r2d2_redis::r2d2::Pool<vm::r2d2_redis::RedisConnectionManager>,
    rocksdb: std::sync::Arc<rocksdb::DB>,
    db_pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    mut block_receiver_in: sync::Receiver<(Block, Vec<Transaction>)>,
    block_sender_out: sync::Sender<(Block, Vec<Transaction>)>,
) {
    let db = db_pool.get().unwrap();
    loop {
        let db2 = db_pool.get().unwrap();
        let mut vm_state = vm::State::new(redis.clone().get().unwrap(), rocksdb.clone());
        let vm_state2 = vm::State::new(redis.get().unwrap(), rocksdb.clone());
        let _redis_connection = redis.get().unwrap();
        if is_block_winner(&mut vm_state, public_key.as_bytes().to_vec()) {
            let ((new_block, transactions), mut vm_state) =
                mine_next_block(redis.clone(), db2, vm_state2).await;
            vm_state.commit();
            new_block.clone().insert(&db, transactions.clone());
            websocket
                .send::<api::Block>((new_block.clone(), transactions.clone()).into())
                .await;
            block_sender_out
                .send((new_block.clone(), transactions.clone()))
                .await;
            println!("Mined block #{}", &new_block.number);
            *BEST_BLOCK.lock().await = Some(new_block.clone());
            continue;
        }
        let (new_block, transactions) = block_receiver_in.next().map(Option::unwrap).await;
        if is_next_block(&new_block).await {
            transaction_processor::apply_block(
                redis.get().unwrap(),
                &mut vm_state,
                new_block.clone(),
                transactions.clone(),
            )
            .await;
            vm_state.commit();
            new_block.clone().insert(&db, transactions.clone());
            websocket
                .send::<api::Block>((new_block.clone(), transactions).into())
                .await;
            println!("Applied block #{}", &new_block.number);
            *BEST_BLOCK.lock().await = Some(new_block.clone());
        }
    }
}
