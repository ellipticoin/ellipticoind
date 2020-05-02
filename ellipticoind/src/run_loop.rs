use crate::api;
use crate::miner::mine_next_block;
use crate::models::{is_block_winner, is_next_block};
use crate::models::{Block, Transaction};
use crate::network::Message;
use crate::transaction_processor;
use crate::BEST_BLOCK;
use async_std::sync;
use diesel::r2d2::{ConnectionManager, Pool};
use ed25519_dalek::PublicKey;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use futures::channel::mpsc;
use futures_util::sink::SinkExt;
use crate::constants::TOKEN_CONTRACT;
use vm::state::db_key;
use serde_bytes::ByteBuf;
use std::collections::HashMap;

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
        println!("1");
        let db2 = db_pool.get().unwrap();
        let mut vm_state = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
        let vm_state2 = vm::State::new(redis.get_connection().unwrap(), rocksdb.clone());
        let mut redis_connection = redis.get_connection().unwrap();
        if is_block_winner(&mut vm_state, public_key.as_bytes().to_vec()) {
            println!("2");
            let ((new_block, transactions), mut vm_state) =
                mine_next_block(&mut redis_connection, db2, vm_state2).await;
            println!("2.5");
            vm_state.commit();
            new_block.clone().insert(&db, transactions.clone());
            websocket
                .send::<api::Block>((&new_block, &transactions).into())
                .await;
            println!("2.6");
            network_sender
                .send(Message::Block((new_block.clone(), transactions.clone())))
                .await.unwrap();
            println!("2.7");
            println!("Mined block #{}", &new_block.number);
            *BEST_BLOCK.lock().await = Some(new_block.clone());
            continue;
        }
        println!("3");
        let (new_block, transactions) = new_block_receiver.next().map(Option::unwrap).await;
        println!("4");
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

            let miners: HashMap<ByteBuf, (u64, ByteBuf)> = serde_cbor::from_slice(&rocksdb
                                                                                  .get(db_key(
                                                                                          &TOKEN_CONTRACT,
                                                                                          &vec![Namespace::Miners as u8]
                                                                                          ))
                                                                                  .unwrap()
                                                                                  .unwrap()).unwrap();
            for (miner, (value, random)) in  miners {
                println!("{:?} {}", base64::encode(&miner), value);
            }
            *BEST_BLOCK.lock().await = Some(new_block.clone());
        }
    }
}
