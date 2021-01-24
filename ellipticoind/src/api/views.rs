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
    pub transaction_number: u32,
    return_value: serde_cbor::Value,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
}
