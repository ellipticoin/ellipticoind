extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;

use crate::constants::TOKEN_CONTRACT;
use crate::diesel::QueryDsl;
use crate::models::*;
use crate::schema::blocks::dsl::blocks;
use crate::schema::hash_onion::dsl::*;
use crate::transaction_processor::{run_transaction, run_transactions, PUBLIC_KEY};
use crate::BEST_BLOCK;
use diesel::dsl::sql_query;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use serde_cbor::Value;

pub fn get_best_block(db: &PgConnection) -> Option<Block> {
    blocks
        .order(crate::schema::blocks::dsl::number.desc())
        .first(db)
        .optional()
        .unwrap()
}

pub async fn next_block_template() -> Block {
    BEST_BLOCK.lock().await.as_ref().map_or(
        Block {
            number: 0,
            ..Default::default()
        },
        |Block { number, hash, .. }| Block {
            parent_hash: Some(hash.to_vec()),
            number: number + 1,
            ..Default::default()
        },
    )
}

pub async fn mine_next_block(
    con: crate::vm::r2d2_redis::r2d2::Pool<crate::vm::r2d2_redis::RedisConnectionManager>,
    pg_db: PooledConnection<ConnectionManager<PgConnection>>,
    rocksdb: std::sync::Arc<rocksdb::DB>,
) -> (Block, Vec<Transaction>) {
    let mut vm_state = crate::vm::State::new(con.get().unwrap(), rocksdb);
    let mut block = next_block_template().await;
    block.winner = PUBLIC_KEY.to_vec();
    let mut transactions = run_transactions(con.clone(), &mut vm_state, &block).await;

    let sender_nonce = random();
    let skin: Vec<Value> = hash_onion
        .select(layer)
        .order(id.desc())
        .first::<Vec<u8>>(&pg_db)
        .unwrap()
        .into_iter()
        .map(|n| n.into())
        .collect();
    let reveal_transaction = crate::vm::Transaction {
        contract_address: TOKEN_CONTRACT.to_vec(),
        sender: PUBLIC_KEY.to_vec(),
        nonce: sender_nonce,
        function: "reveal".to_string(),
        arguments: vec![skin.into()],
        gas_limit: 10000000,
    };
    let reveal_result = run_transaction(&mut vm_state, &reveal_transaction, &block);
    sql_query(
        "delete from hash_onion where id in (
        select id from hash_onion order by id desc limit 1
    )",
    )
    .execute(&pg_db)
    .unwrap();
    transactions.push(reveal_result);
    block.set_hash();
    transactions.iter_mut().for_each(|transaction| {
        transaction.set_hash();
        transaction.block_hash = block.hash.clone();
    });
    vm_state.commit();
    block.clone().insert(&pg_db, transactions.clone());
    (block, transactions)
}

fn random() -> u64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen_range(3000, 5000)
}
