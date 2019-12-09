use crate::diesel::RunQueryDsl;
use crate::helpers::sha256;
use crate::schema::blocks;
use crate::schema::transactions;
use diesel::PgConnection;
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
    arguments: Vec<serde_cbor::Value>,
    #[serde(with = "serde_bytes")]
    pub block_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    contract_address: Vec<u8>,
    function: String,
    gas_limit: u64,
    nonce: u64,
    return_code: u64,
    return_value: serde_cbor::Value,
    #[serde(with = "serde_bytes")]
    sender: Vec<u8>,
}

impl From<Transaction> for TransactionWithoutHash {
    fn from(transaction: Transaction) -> Self {
        Self {
            block_hash: transaction.block_hash,
            contract_address: transaction.contract_address,
            sender: transaction.sender,
            gas_limit: transaction.gas_limit as u64,
            nonce: transaction.nonce as u64,
            function: transaction.function,
            arguments: serde_cbor::from_slice(&transaction.arguments).unwrap(),
            return_code: transaction.return_code as u64,
            return_value: serde_cbor::from_slice(&transaction.return_value).unwrap(),
        }
    }
}


impl Transaction {
    pub fn set_hash(&mut self) {
        self.hash = sha256(serde_cbor::to_vec(&TransactionWithoutHash::from(self.clone())).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let thing1 = Thing1 {
            aa: vec![1, 2, 3],
            b: vec![1, 2, 3],
        };
        let thing2 = Thing1 {
            b: vec![1, 2, 3],
            aa: vec![1, 2, 3],
        };
        assert_eq!(serde_cbor::to_vec(&thing1).unwrap(), serde_cbor::to_vec(&thing2).unwrap());
    }
}
