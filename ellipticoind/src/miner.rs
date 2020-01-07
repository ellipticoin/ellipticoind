extern crate rocksdb;
extern crate serde;
extern crate serde_cbor;
extern crate vm;

use crate::api;
use crate::models::*;
use crate::network::Message;
use crate::schema::blocks::dsl::blocks;
use crate::transaction_processor::{run_transactions, PUBLIC_KEY};
use async_std::task::sleep;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use hashfactor::hashfactor;
use std::time::Duration;
use vm::Changeset;

const HASHFACTOR_TARGET: u64 = 1;

pub fn get_best_block(db: &PgConnection) -> Option<Block> {
    blocks
        .order(crate::schema::blocks::dsl::number.desc())
        .first(db)
        .optional()
        .unwrap()
}

pub fn next_block(best_block: &Option<Block>) -> Block {
    best_block.as_ref().map_or(
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
    api: &mut api::State,
    vm_state: &mut vm::State,
    best_block: Option<Block>,
) -> (Changeset, Changeset, Block, Vec<Transaction>) {
    let mut block = next_block(&best_block);
    block.winner = PUBLIC_KEY.to_vec();
    let mut transactions = run_transactions(api, vm_state, &block).await;

    let rand = random();
    sleep(Duration::from_millis(rand)).await;
    let encoded_block = serde_cbor::to_vec(&UnminedBlock::from(&block)).unwrap();
    block.proof_of_work_value = hashfactor(encoded_block, HASHFACTOR_TARGET) as i64;
    block.set_hash();
    transactions.iter_mut().for_each(|transaction| {
        transaction.set_hash();
        transaction.block_hash = block.hash.clone();
    });
    api.broadcast(&Message::Block((block.clone(), transactions.clone())))
        .await;
    (
        vm_state.memory_changeset.clone(),
        vm_state.storage_changeset.clone(),
        block,
        transactions,
    )
}

fn random() -> u64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen_range(1000, 2000)
}
