use crate::api;
use crate::miner::mine_next_block;

use crate::models::{is_block_winner, is_next_block};
use crate::network::Message;
use crate::transaction_processor;
use crate::vm::{self, redis::Commands};
use crate::BEST_BLOCK;
use async_std::sync;

use futures::future::FutureExt;
use futures::stream::StreamExt;

pub async fn run(
    mut websocket: api::websocket::Websocket,
    redis: vm::r2d2_redis::r2d2::Pool<vm::r2d2_redis::RedisConnectionManager>,
    rocksdb: std::sync::Arc<rocksdb::DB>,
    db_pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    mut receiver_in: sync::Receiver<Message>,
    sender_out: sync::Sender<Message>,
) {
    loop {
        let mut vm_state = vm::State::new(redis.clone().get().unwrap(), rocksdb.clone());
        if is_block_winner(&mut vm_state) {
            let (new_block, transactions) =
                mine_next_block(redis.clone(), db_pool.get().unwrap(), rocksdb.clone()).await;
            websocket
                .send::<api::Block>((new_block.clone(), transactions.clone()).into())
                .await;
            sender_out
                .send(Message::Block((new_block.clone(), transactions.clone())))
                .await;
            *BEST_BLOCK.lock().await = Some(new_block.clone());
            println!("Mined block #{}", &new_block.number);
            continue;
        }
        match receiver_in.next().map(Option::unwrap).await {
            Message::Block((new_block, transactions)) => {
                if is_next_block(&new_block).await {
                    transaction_processor::apply_block(
                        redis.get().unwrap(),
                        &mut vm_state,
                        new_block.clone(),
                        transactions.clone(),
                        db_pool.get().unwrap(),
                    )
                    .await;
                    websocket
                        .send::<api::Block>((new_block.clone(), transactions).into())
                        .await;
                    println!("Applied block #{}", &new_block.number);
                    *BEST_BLOCK.lock().await = Some(new_block.clone());
                }
            }
            Message::Transaction(transaction) => {
                redis
                    .get()
                    .unwrap()
                    .rpush::<&str, Vec<u8>, ()>(
                        "transactions::pending",
                        serde_cbor::to_vec(&transaction).unwrap(),
                    )
                    .unwrap();
            }
        }
    }
}
