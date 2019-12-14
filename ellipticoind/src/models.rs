use crate::diesel::ExpressionMethods;
use crate::diesel::RunQueryDsl;
use crate::helpers::sha256;
use crate::schema::blocks;
use crate::schema::transactions;
use crate::schema::transactions::columns::{nonce, sender};
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::{OptionalExtension, PgConnection, QueryDsl};
use serde::Serialize;

#[derive(Queryable, Insertable, Default, Clone, Debug, Serialize)]
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
            arguments: serde_cbor::to_vec(&transaction.arguments).unwrap(),
            return_code: transaction.return_code as i64,
            return_value: serde_cbor::to_vec(&transaction.return_value).unwrap(),
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
        self.hash = sha256(serde_cbor::to_vec(&BlockWithoutHash::from(self.clone())).unwrap());
    }

    pub fn insert(&self, db: &PgConnection) {
        diesel::dsl::insert_into(crate::schema::blocks::dsl::blocks)
            .values(self)
            .execute(db)
            .unwrap();
    }
}

#[derive(Insertable, Queryable, Clone, Debug, Serialize)]
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
            arguments: serde_cbor::from_slice(&transaction.arguments).unwrap(),
        }
    }
}

impl Transaction {
    pub fn set_hash(&mut self) {
        self.hash =
            sha256(serde_cbor::to_vec(&TransactionWithoutHash::from(self.clone())).unwrap());
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
    crate::schema::transactions::dsl::transactions
        .order(nonce.desc())
        .filter(sender.eq(address))
        .select(nonce)
        .first(con)
        .optional()
        .unwrap()
}
