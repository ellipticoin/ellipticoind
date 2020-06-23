use crate::{
    config::keypair,
    constants::{Namespace, TOKEN_CONTRACT},
    diesel::RunQueryDsl,
    helpers::sha256,
    schema::blocks,
};

use crate::schema::transactions;

use crate::{vm, BEST_BLOCK};
use diesel::dsl::insert_into;

use diesel::PgConnection;
use serde::{Deserialize, Serialize};
use serde_cbor::to_vec;

#[derive(Queryable, Identifiable, Insertable, Default, Clone, Debug, Serialize, Deserialize)]
#[primary_key(hash)]
pub struct Block {
    pub hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    pub winner: Vec<u8>,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
}

#[derive(Serialize)]
pub struct BlockWithoutHash {
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    pub winner: Vec<u8>,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
}

#[derive(Serialize)]
pub struct UnminedBlock {
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    pub winner: Vec<u8>,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
}

pub async fn is_next_block(block: &Block) -> bool {
    if let Some(best_block) = BEST_BLOCK.lock().await.clone() {
        block.number > best_block.number
    } else {
        true
    }
}

pub fn is_block_winner(vm_state: &mut vm::State) -> bool {
    let winner = vm_state.get_storage(&TOKEN_CONTRACT, &vec![Namespace::CurrentMiner as u8]);
    winner.eq(&keypair().public.as_bytes().to_vec())
}

impl From<&Block> for UnminedBlock {
    fn from(block: &Block) -> Self {
        Self {
            parent_hash: block.parent_hash.clone(),
            number: block.number,
            winner: block.winner.clone(),
            memory_changeset_hash: block.memory_changeset_hash.clone(),
            storage_changeset_hash: block.storage_changeset_hash.clone(),
        }
    }
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
    pub fn set_hash(&mut self) {
        self.hash = sha256(to_vec(&BlockWithoutHash::from(self.clone())).unwrap());
    }

    pub fn insert(self, db: &PgConnection, transactions: Vec<crate::models::Transaction>) {
        insert_into(blocks::dsl::blocks)
            .values(&self)
            .execute(db)
            .unwrap();
        insert_into(transactions::dsl::transactions)
            .values(&transactions)
            .execute(db)
            .expect(&format!(
                "{:?}",
                transactions
                    .iter()
                    .map(|t| (t.function.clone(), base64::encode(&t.sender.clone())))
                    .collect::<Vec<(String, String)>>()
            ));
    }
}
