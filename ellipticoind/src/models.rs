use crate::constants::{CURRENT_MINER_ENUM, TOKEN_CONTRACT};
use crate::diesel::ExpressionMethods;
use crate::diesel::RunQueryDsl;
use crate::helpers::sha256;
use crate::schema::blocks;
use crate::schema::hash_onion;
use crate::schema::transactions;
use crate::schema::transactions::columns::{nonce, sender};
use crate::BEST_BLOCK;
use diesel::dsl::insert_into;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::{OptionalExtension, PgConnection, QueryDsl};
use serde::{Deserialize, Serialize};
use serde_cbor::{from_slice, to_vec};

#[derive(Queryable, Identifiable, Insertable, Default, Clone, Debug, Serialize, Deserialize)]
#[primary_key(hash)]
pub struct Block {
    pub hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    pub winner: Vec<u8>,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
    pub proof_of_work_value: i64,
}

#[derive(Serialize)]
pub struct BlockWithoutHash {
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    pub winner: Vec<u8>,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
    pub proof_of_work_value: i64,
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

pub fn is_block_winner(vm_state: &mut vm::State, public_key: Vec<u8>) -> bool {
    vm_state
        .get_storage(&TOKEN_CONTRACT, &CURRENT_MINER_ENUM)
        .eq(&public_key)
}

impl From<Transaction> for vm::Transaction {
    fn from(transaction: Transaction) -> vm::Transaction {
        vm::Transaction {
            sender: transaction.sender,
            arguments: from_slice(&transaction.arguments).unwrap(),
            contract_address: transaction.contract_address,
            function: transaction.function,
            gas_limit: transaction.gas_limit as u64,
            nonce: transaction.nonce as u64,
        }
    }
}

impl From<vm::CompletedTransaction> for Transaction {
    fn from(transaction: vm::CompletedTransaction) -> Self {
        Self {
            hash: vec![],
            block_hash: vec![],
            contract_address: transaction.contract_address,
            sender: transaction.sender,
            gas_limit: transaction.gas_limit as i64,
            nonce: transaction.nonce as i64,
            function: transaction.function,
            arguments: to_vec(&transaction.arguments).unwrap(),
            return_code: transaction.return_code as i64,
            return_value: to_vec(&transaction.return_value).unwrap(),
        }
    }
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
            proof_of_work_value: block.proof_of_work_value.clone(),
        }
    }
}

impl Block {
    pub fn set_hash(&mut self) {
        self.hash = sha256(to_vec(&BlockWithoutHash::from(self.clone())).unwrap());
    }

    pub fn insert(self, db: &PgConnection, transactions: Vec<Transaction>) {
        insert_into(blocks::dsl::blocks)
            .values(&self)
            .execute(db)
            .unwrap();
        insert_into(transactions::dsl::transactions)
            .values(&transactions)
            .execute(db)
            .expect(&format!("{:?}", transactions.iter().map(|t| (t.function.clone(), base64::encode(&t.sender.clone()))).collect::<Vec<(String, String)>>()));
    }
}

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    Associations,
    PartialEq,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[primary_key(hash)]
#[belongs_to(Block, foreign_key = "block_hash")]
pub struct Transaction {
    pub block_hash: Vec<u8>,
    pub hash: Vec<u8>,
    pub contract_address: Vec<u8>,
    pub sender: Vec<u8>,
    pub gas_limit: i64,
    pub nonce: i64,
    pub function: String,
    pub arguments: Vec<u8>,
    pub return_code: i64,
    pub return_value: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct TransactionWithoutHash {
    nonce: u64,
    #[serde(with = "serde_bytes")]
    sender: Vec<u8>,
    function: String,
    arguments: Vec<serde_cbor::Value>,
    gas_limit: u64,
    #[serde(with = "serde_bytes")]
    contract_address: Vec<u8>,
}

impl From<Transaction> for TransactionWithoutHash {
    fn from(transaction: Transaction) -> Self {
        Self {
            contract_address: transaction.contract_address,
            sender: transaction.sender,
            gas_limit: transaction.gas_limit as u64,
            nonce: transaction.nonce as u64,
            function: transaction.function,
            arguments: from_slice(&transaction.arguments).unwrap(),
        }
    }
}

impl Transaction {
    pub fn set_hash(&mut self) {
        self.hash = sha256(to_vec(&TransactionWithoutHash::from(self.clone())).unwrap());
    }
}

pub fn next_nonce(
    con: &PooledConnection<ConnectionManager<PgConnection>>,
    address: Vec<u8>,
) -> u64 {
    (highest_nonce(&con, address).unwrap_or(-1) + 1) as u64
}
pub fn highest_nonce(
    con: &PooledConnection<ConnectionManager<PgConnection>>,
    address: Vec<u8>,
) -> Option<i64> {
    transactions::dsl::transactions
        .order(nonce.desc())
        .filter(sender.eq(address))
        .select(nonce)
        .first(con)
        .optional()
        .unwrap()
}

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    Associations,
    PartialEq,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[primary_key(layer)]
#[table_name = "hash_onion"]
pub struct HashOnion {
    pub layer: Vec<u8>,
}
