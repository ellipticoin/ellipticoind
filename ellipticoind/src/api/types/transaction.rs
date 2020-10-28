use crate::api::types::misc::{Bytes, U32, U64};
use crate::models;

#[derive(Clone, Debug)]
pub struct Transaction {
    pub id: U32,
    pub network_id: U64,
    pub block_number: U32,
    pub position: U32,
    pub contract: String,
    pub sender: Bytes,
    pub nonce: U32,
    pub function: String,
    pub arguments: Bytes,
    pub return_value: Bytes,
    pub raw: Bytes,
}

#[juniper::graphql_object]
impl Transaction {
    fn id(&self) -> U32 {
        self.id.clone()
    }

    fn network_id(&self) -> U64 {
        self.network_id.clone()
    }

    fn block_number(&self) -> U32 {
        self.block_number.clone()
    }

    fn position(&self) -> U32 {
        self.position.clone()
    }

    fn contract(&self) -> String {
        self.contract.clone()
    }

    fn sender(&self) -> Bytes {
        self.sender.clone()
    }

    fn nonce(&self) -> U32 {
        self.nonce.clone()
    }

    fn function(&self) -> String {
        self.function.clone()
    }

    fn arguments(&self) -> Bytes {
        self.arguments.clone()
    }

    fn return_value(&self) -> Bytes {
        self.return_value.clone()
    }

    fn raw(&self) -> Bytes {
        self.raw.clone()
    }
}

impl From<models::Transaction> for Transaction {
    fn from(transaction: models::Transaction) -> Self {
        Self {
            id: U32(transaction.id as u32),
            network_id: U64(transaction.network_id as u64),
            nonce: U32(transaction.nonce as u32),
            position: U32(transaction.position as u32),
            block_number: U32(transaction.block_number as u32),
            function: transaction.function,
            sender: Bytes(transaction.sender),
            contract: transaction.contract,
            arguments: transaction.arguments.into(),
            return_value: Bytes(transaction.return_value),
            raw: Bytes(transaction.raw),
        }
    }
}
