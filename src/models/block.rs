use crate::{
    config::{get_pg_connection, public_key},
    constants::TOKEN_CONTRACT,
    diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::{bytes_to_value, sha256},
    models::{self, HashOnion, Transaction},
    schema::{blocks, blocks::dsl},
    vm,
    vm::State,
    IS_CURRENT_MINER,
};
use diesel::dsl::insert_into;
use serde::{Deserialize, Serialize};
use serde_cbor::to_vec;

#[derive(Queryable, Identifiable, Insertable, Clone, Debug, Serialize, Deserialize)]
#[primary_key(hash)]
pub struct Block {
    pub hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub winner: Vec<u8>,
    pub number: i64,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
    pub sealed: bool,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            hash: vec![],
            parent_hash: Some(vec![]),
            number: 0,
            winner: public_key(),
            memory_changeset_hash: vec![],
            storage_changeset_hash: vec![],
            sealed: false,
        }
    }
}

#[derive(Serialize)]
pub struct BlockWithoutHash {
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    pub winner: Vec<u8>,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
}

impl From<Block> for BlockWithoutHash {
    fn from(block: Block) -> Self {
        Self {
            parent_hash: block.parent_hash.clone(),
            number: block.number,
            winner: block.winner.clone(),
            memory_changeset_hash: block.memory_changeset_hash.clone(),
            storage_changeset_hash: block.storage_changeset_hash.clone(),
        }
    }
}

impl Block {
    pub fn new(number: i64) -> Self {
        let mut block = Self {
            hash: vec![],
            number,
            winner: public_key(),
            memory_changeset_hash: vec![],
            storage_changeset_hash: vec![],
            parent_hash: Some(vec![]),
            sealed: false,
        };
        block.set_hash();
        block
    }

    pub fn apply(mut self, vm_state: &mut State, transactions: Vec<models::Transaction>) {
        let pg_db = get_pg_connection();
        self.set_hash();
        self.sealed = true;
        insert_into(dsl::blocks)
            .values(&self)
            .execute(&pg_db)
            .unwrap();
        transactions.iter().for_each(|transaction| {
            Transaction::run(
                vm_state,
                &self,
                vm::Transaction::from(transaction),
                transaction.position,
            );
        });
        vm_state.commit();
        println!("Applied block #{}", self.number);
    }

    pub async fn insert(vm_state: &mut State) -> Block {
        let pg_db = get_pg_connection();
        let block = Self::new(vm_state.block_number() as i64);
        insert_into(dsl::blocks)
            .values(&block)
            .execute(&pg_db)
            .unwrap();
        block
    }

    pub async fn is_valid(&self) -> bool {
        true
    }

    pub async fn seal(&self, vm_state: &mut State, transaction_position: i64) -> Vec<Transaction> {
        let pg_db = get_pg_connection();
        let reveal_transaction = vm::Transaction::new(
            TOKEN_CONTRACT.to_vec(),
            "reveal",
            vec![bytes_to_value(HashOnion::peel(&pg_db))],
        );
        // let current_block = CURRENT_BLOCK.lock().await.as_ref().unwrap().clone();
        Transaction::run(vm_state, &self, reveal_transaction, transaction_position);
        *IS_CURRENT_MINER.lock().await = vm_state.current_miner().map_or(false, |current_miner| {
            current_miner.address.eq(&public_key())
        });
        diesel::update(dsl::blocks.filter(dsl::hash.eq(self.hash.clone())))
            .set(dsl::sealed.eq(true))
            .execute(&pg_db)
            .unwrap();
        Transaction::belonging_to(self)
            .load::<Transaction>(&pg_db)
            .unwrap()
    }

    pub fn set_hash(&mut self) {
        self.hash = sha256(to_vec(&BlockWithoutHash::from(self.clone())).unwrap());
    }
}
