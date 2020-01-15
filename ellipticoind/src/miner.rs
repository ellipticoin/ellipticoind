extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate vm;

use crate::models::*;
use crate::schema::blocks::dsl::blocks;
use crate::transaction_processor::{run_transactions, PUBLIC_KEY};
use crate::BEST_BLOCK;
use async_std::task::sleep;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use hashfactor::hashfactor;
use std::time::Duration;

const HASHFACTOR_TARGET: u64 = 1;

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
    mut vm_state: vm::State,
) -> ((Block, Vec<Transaction>), vm::State) {
    let mut block = next_block_template().await;
    block.winner = PUBLIC_KEY.to_vec();
    let mut transactions = run_transactions(con, &mut vm_state, &block).await;

    let rand = random();
    sleep(Duration::from_millis(rand)).await;
    let encoded_block = serde_cbor::to_vec(&UnminedBlock::from(&block)).unwrap();
    block.proof_of_work_value = hashfactor(encoded_block, HASHFACTOR_TARGET) as i64;
    block.set_hash();
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
