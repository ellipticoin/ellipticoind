use crate::{
    config::public_key,
    constants::TOKEN_CONTRACT,
    diesel::QueryDsl,
    helpers::bytes_to_value,
    models::*,
    schema,
    schema::blocks::dsl::blocks,
    transaction_processor::{run_transaction, run_transactions},
    vm,
    vm::redis,
    BEST_BLOCK,
};
use diesel::{
    pg::PgConnection,
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};

pub fn get_best_block(db: &PgConnection) -> Option<Block> {
    blocks
        .order(schema::blocks::dsl::number.desc())
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
    con: redis::Pool,
    pg_db: PooledConnection<ConnectionManager<PgConnection>>,
    rocksdb: std::sync::Arc<rocksdb::DB>,
) -> (Block, Vec<Transaction>) {
    let mut vm_state = vm::State::new(con.get().unwrap(), rocksdb);
    let mut block = next_block_template().await;
    block.winner = public_key();
    let mut transactions = run_transactions(con.clone(), &mut vm_state, &block).await;
    let reveal_transaction = vm::Transaction::new(
        TOKEN_CONTRACT.to_vec(),
        "reveal",
        vec![bytes_to_value(HashOnion::peel(&pg_db))],
    );
    let reveal_result = run_transaction(&mut vm_state, &reveal_transaction, &block);
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
