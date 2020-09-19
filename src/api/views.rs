use crate::models;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub number: u32,
    #[serde(with = "serde_bytes")]
    pub memory_changeset_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub storage_changeset_hash: Vec<u8>,
    pub transactions: Vec<Transaction>,
    pub sealed: bool,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Transaction {
    pub id: i32,
    pub block_number: i32,
    pub network_id: i64,
    pub arguments: Vec<serde_cbor::Value>,
    pub position: u32,
    pub contract: String,
    pub function: String,
    pub nonce: u32,
    return_value: serde_cbor::Value,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
}

impl From<(models::Block, Vec<models::Transaction>)> for Block {
    fn from(block: (models::Block, Vec<models::Transaction>)) -> Self {
        Self {
            number: block.0.number as u32,
            memory_changeset_hash: block.0.memory_changeset_hash.clone(),
            storage_changeset_hash: block.0.storage_changeset_hash.clone(),
            sealed: block.0.sealed,
            transactions: block
                .1
                .into_iter()
                .map(Transaction::from)
                .collect::<Vec<Transaction>>(),
        }
    }
}

impl From<models::Transaction> for Transaction {
    fn from(transaction: models::Transaction) -> Self {
        Self {
            id: transaction.id as i32,
            block_number: transaction.block_number,
            network_id: transaction.network_id as i64,
            contract: transaction.contract.clone(),
            position: transaction.position as u32,
            sender: transaction.sender.clone(),
            nonce: transaction.nonce as u32,
            function: transaction.function.clone(),
            arguments: serde_cbor::from_slice(&transaction.arguments).unwrap(),
            return_value: serde_cbor::from_slice(&transaction.return_value).unwrap(),
        }
    }
}

impl From<Block> for (models::Block, Vec<models::Transaction>) {
    fn from(block: Block) -> Self {
        (
            models::Block {
                number: block.number as i32,
                memory_changeset_hash: block.memory_changeset_hash.clone(),
                storage_changeset_hash: block.storage_changeset_hash.clone(),
                sealed: block.sealed,
            },
            block
                .transactions
                .into_iter()
                .map(models::Transaction::from)
                .collect(),
        )
    }
}

impl From<Transaction> for models::Transaction {
    fn from(transaction: Transaction) -> Self {
        Self {
            id: transaction.id,
            block_number: transaction.block_number,
            network_id: transaction.network_id as i64,
            contract: transaction.contract.clone(),
            position: transaction.position as i32,
            sender: transaction.sender.clone(),
            nonce: transaction.nonce as i32,
            function: transaction.function.clone(),
            arguments: serde_cbor::to_vec(&transaction.arguments).unwrap(),
            return_value: serde_cbor::to_vec(&transaction.return_value).unwrap(),
            raw: vec![],
        }
    }
}
