extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate vm;

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
            number: 1,
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
    con: &mut vm::Connection,
    pg_db: PooledConnection<ConnectionManager<PgConnection>>,
    mut vm_state: vm::State,
) -> ((Block, Vec<Transaction>), vm::State) {
    println!("a");
    let mut block = next_block_template().await;
    block.winner = PUBLIC_KEY.to_vec();
    let mut transactions = run_transactions(con, &mut vm_state, &block).await;

    println!("b");
    let sender_nonce = random();
    let skin: Vec<u8> = hash_onion
        .select(layer)
        .order(id.desc())
        .first(&pg_db)
        .unwrap();
    println!("c");
    let reveal_transaction = vm::Transaction {
        contract_address: TOKEN_CONTRACT.to_vec(),
        sender: PUBLIC_KEY.to_vec(),
        nonce: sender_nonce,
        function: "reveal".to_string(),
        arguments: vec![Value::Bytes(skin.clone().into())],
        gas_limit: 10000000,
    };
    println!("d");
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
    println!("e");
    transactions.iter_mut().for_each(|transaction| {
        transaction.set_hash();
        transaction.block_hash = block.hash.clone();
    });
    ((block, transactions), vm_state)
}

fn random() -> u64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen_range(3000, 5000)
}
