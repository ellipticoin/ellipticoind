use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    #[serde(with = "serde_bytes")]
    pub hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    #[serde(with = "serde_bytes")]
    pub winner: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub memory_changeset_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub storage_changeset_hash: Vec<u8>,
    pub proof_of_work_value: i64,
    pub transactions: Vec<Transaction>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Transaction {
    pub network_id: u64,
    pub arguments: Vec<serde_cbor::Value>,
    #[serde(with = "serde_bytes")]
    pub block_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub contract_address: Vec<u8>,
    pub function: String,
    pub gas_limit: u64,
    pub nonce: u64,
    return_value: serde_cbor::Value,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
}

impl From<(crate::models::Block, Vec<crate::models::Transaction>)> for Block {
    fn from(block: (crate::models::Block, Vec<crate::models::Transaction>)) -> Self {
        Self {
            hash: block.0.hash.clone(),
            parent_hash: block.0.parent_hash.clone(),
            number: block.0.number,
            winner: block.0.winner.clone(),
            memory_changeset_hash: block.0.memory_changeset_hash.clone(),
            storage_changeset_hash: block.0.storage_changeset_hash.clone(),
            proof_of_work_value: block.0.proof_of_work_value.clone(),
            transactions: block
                .1
                .into_iter()
                .map(Transaction::from)
                .collect::<Vec<Transaction>>(),
        }
    }
}

impl From<crate::models::Transaction> for Transaction {
    fn from(transaction: crate::models::Transaction) -> Self {
        Self {
            network_id: transaction.network_id as u64,
            contract_address: transaction.contract_address.clone(),
            block_hash: transaction.block_hash.clone(),
            sender: transaction.sender.clone(),
            nonce: transaction.nonce as u64,
            gas_limit: transaction.gas_limit as u64,
            function: transaction.function.clone(),
            arguments: serde_cbor::from_slice(&transaction.arguments).unwrap(),
            return_value: serde_cbor::from_slice(&transaction.return_value).unwrap(),
        }
    }
}

impl From<Block> for (crate::models::Block, Vec<crate::models::Transaction>) {
    fn from(block: Block) -> Self {
        (
            crate::models::Block {
                hash: block.hash.clone(),
                parent_hash: block.parent_hash.clone(),
                number: block.number,
                winner: block.winner.clone(),
                memory_changeset_hash: block.memory_changeset_hash.clone(),
                storage_changeset_hash: block.storage_changeset_hash.clone(),
                proof_of_work_value: block.proof_of_work_value.clone(),
            },
            block
                .transactions
                .into_iter()
                .map(crate::models::Transaction::from)
                .collect(),
        )
    }
}

impl From<Transaction> for crate::models::Transaction {
    fn from(transaction: Transaction) -> Self {
        Self {
            network_id: transaction.network_id as i64,
            hash: vec![],
            contract_address: transaction.contract_address.clone(),
            block_hash: transaction.block_hash.clone(),
            sender: transaction.sender.clone(),
            nonce: transaction.nonce as i64,
            gas_limit: transaction.gas_limit as i64,
            function: transaction.function.clone(),
            arguments: serde_cbor::to_vec(&transaction.arguments).unwrap(),
            return_value: serde_cbor::to_vec(&transaction.return_value).unwrap(),
        }
    }
}
