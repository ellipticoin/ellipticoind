use crate::api;
use crate::constants::TOKEN_CONTRACT;
use crate::miner::mine_next_block;
use crate::models::{is_block_winner, is_next_block};
use crate::models::{Block, Transaction};
use crate::network::Message;
use crate::transaction_processor;
use crate::BEST_BLOCK;
use async_std::sync;
use diesel::r2d2::{ConnectionManager, Pool};
use ed25519_dalek::PublicKey;
use futures::channel::mpsc;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use futures_util::sink::SinkExt;
use serde_bytes::ByteBuf;
use std::collections::HashMap;
use vm::state::db_key;

enum Namespace {
    _Allowences,
    Balances,
    CurrentMiner,
    Miners,
    RandomSeed,
    EthereumBalances,
}

pub async fn run(
    public_key: std::sync::Arc<PublicKey>,
    mut websocket: api::websocket::Websocket,
    mut network_sender: mpsc::Sender<Message>,
    redis: redis::Client,
    rocksdb: std::sync::Arc<rocksdb::DB>,
    db_pool: Pool<ConnectionManager<diesel::PgConnection>>,
    mut new_block_receiver: sync::Receiver<(Block, Vec<Transaction>)>,
) {
    let db = db_pool.get().unwrap();
    loop {
        let db2 = db_pool.get().unwrap();
        let mut vm_state = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
        let vm_state2 = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
        let mut redis_connection = redis.get_connection().unwrap();
        if is_block_winner(&mut vm_state, public_key.as_bytes().to_vec()) {
            let ((new_block, transactions), mut vm_state) =
                mine_next_block(&mut redis_connection, db2, vm_state2).await;
            vm_state.commit();
            new_block.clone().insert(&db, transactions.clone());
            websocket
                .send::<api::Block>((&new_block, &transactions).into())
                .await;
            network_sender
                .send(Message::Block((new_block.clone(), transactions.clone())))
                .await
                .unwrap();
            println!("Mined block #{}", &new_block.number);
            *BEST_BLOCK.lock().await = Some(new_block.clone());
            continue;
        }
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

            let current_miner = &rocksdb
                .get(db_key(
                    &TOKEN_CONTRACT,
                    &vec![Namespace::CurrentMiner as u8],
                ))
                .unwrap()
                .unwrap();
            *BEST_BLOCK.lock().await = Some(new_block.clone());
        }
    }
}
