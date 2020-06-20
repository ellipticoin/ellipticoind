use crate::{
    diesel::{ExpressionMethods, RunQueryDsl},
    helpers::sha256,
    models::block::Block,
    schema::{
        transactions,
        transactions::columns::{nonce, sender},
    },
    vm,
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    OptionalExtension, PgConnection, QueryDsl,
};
use serde::{Deserialize, Serialize};
use serde_cbor::{from_slice, to_vec};

impl From<Transaction> for vm::Transaction {
    fn from(transaction: Transaction) -> vm::Transaction {
        vm::Transaction {
            network_id: transaction.network_id as u32,
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
            network_id: transaction.network_id as i64,
            hash: vec![],
            block_hash: vec![],
            contract_address: transaction.contract_address,
            sender: transaction.sender,
            gas_limit: transaction.gas_limit as i64,
            nonce: transaction.nonce as i64,
            function: transaction.function,
            arguments: to_vec(&transaction.arguments).unwrap(),
            return_value: to_vec(&transaction.return_value).unwrap(),
        }
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
    pub network_id: i64,
    pub block_hash: Vec<u8>,
    pub hash: Vec<u8>,
    pub contract_address: Vec<u8>,
    pub sender: Vec<u8>,
    pub gas_limit: i64,
    pub nonce: i64,
    pub function: String,
    pub arguments: Vec<u8>,
    pub return_value: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct TransactionWithoutHash {
    network_id: u32,
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
            network_id: transaction.network_id as u32,
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
