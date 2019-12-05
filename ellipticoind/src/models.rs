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
    pub hash: Vec<u8>,
    pub block_hash: Vec<u8>,
    contract_address: Vec<u8>,
    sender: Vec<u8>,
    gas_limit: i64,
    nonce: i64,
    function: String,
    arguments: Vec<u8>,
    return_code: i64,
    return_value: Vec<u8>,
}

#[derive(Serialize)]
pub struct TransactionWithoutHash {
    pub block_hash: Vec<u8>,
    contract_address: Vec<u8>,
    sender: Vec<u8>,
    gas_limit: i64,
    nonce: i64,
    function: String,
    arguments: Vec<u8>,
    return_code: i64,
    return_value: Vec<u8>,
}

impl From<Transaction> for TransactionWithoutHash {
    fn from(transaction: Transaction) -> Self {
        Self {
            block_hash: transaction.block_hash,
            contract_address: transaction.contract_address,
            sender: transaction.sender,
            gas_limit: transaction.gas_limit,
            nonce: transaction.nonce,
            function: transaction.function,
            arguments: transaction.arguments,
            return_code: transaction.return_code,
            return_value: transaction.return_value,
        }
    }
}

impl Transaction {
    pub fn set_hash(&mut self) {
        self.hash = sha256(serde_cbor::to_vec(&TransactionWithoutHash::from(self.clone())).unwrap());
    }
}
