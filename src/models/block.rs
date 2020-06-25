use crate::{
    config::{get_pg_connection, public_key},
    constants::TOKEN_CONTRACT,
    diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::{bytes_to_value, sha256},
    models::{self, HashOnion, Transaction},
    schema::{blocks, blocks::dsl},
    vm, CURRENT_BLOCK, VM_STATE,
};
use diesel::dsl::insert_into;
use serde::{Deserialize, Serialize};
use serde_cbor::to_vec;

#[derive(Queryable, Identifiable, Insertable, Clone, Debug, Serialize, Deserialize)]
#[primary_key(hash)]
pub struct Block {
    pub hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    pub winner: Vec<u8>,
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

    pub async fn apply(mut self, transactions: Vec<models::Transaction>) {
        let pg_db = get_pg_connection();
        self.set_hash();
        self.sealed = true;
        insert_into(dsl::blocks)
            .values(&self)
            .execute(&pg_db)
            .unwrap();
        *CURRENT_BLOCK.lock().await = Some(self.clone());
        let mut vm_state = VM_STATE.lock().await;
        transactions.iter().for_each(|transaction| {
            Transaction::run(&mut vm_state, &self, vm::Transaction::from(transaction));
        });
        vm_state.commit();
        println!("Applied block #{}", self.number);
    }

    pub async fn insert() -> Block {
        let pg_db = get_pg_connection();
        let mut vm_state = VM_STATE.lock().await;
        let block = Self::new(vm_state.block_number() as i64);
        insert_into(dsl::blocks)
            .values(&block)
            .execute(&pg_db)
            .unwrap();
        *CURRENT_BLOCK.lock().await = Some(block.clone());
        block
    }

    pub async fn is_valid(&self) -> bool {
        let current_block = CURRENT_BLOCK.lock().await.as_ref().unwrap().clone();
        self.number == current_block.number + 1
    }

    pub async fn seal(&self) -> Vec<Transaction> {
        let pg_db = get_pg_connection();
        let reveal_transaction = vm::Transaction::new(
            TOKEN_CONTRACT.to_vec(),
            "reveal",
            vec![bytes_to_value(HashOnion::peel(&pg_db))],
        );
        let mut vm_state = VM_STATE.lock().await;
        let current_block = CURRENT_BLOCK.lock().await.as_ref().unwrap().clone();
        Transaction::run(&mut vm_state, &current_block, reveal_transaction);
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
