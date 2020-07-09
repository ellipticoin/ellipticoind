use crate::{
    config::get_pg_connection,
    diesel::{ExpressionMethods, RunQueryDsl},
    helpers::sha256,
    models,
    models::block::Block,
    schema::{
        transactions,
        transactions::{
            columns::{nonce, sender},
            dsl::transactions as transactions_table,
        },
    },
    vm,
};
use diesel::{
    insert_into,
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
            gas_limit: transaction.gas_limit as u32,
            nonce: transaction.nonce as u32,
            signature: Some(transaction.signature),
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
    pub position: i64,
    pub contract_address: Vec<u8>,
    pub sender: Vec<u8>,
    pub gas_limit: i64,
    pub gas_used: i64,
    pub nonce: i64,
    pub function: String,
    pub arguments: Vec<u8>,
    pub return_value: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct TransactionWithoutHash {
    nonce: u32,
    #[serde(with = "serde_bytes")]
    sender: Vec<u8>,
    function: String,
    gas_used: u64,
    position: u32,
    arguments: Vec<serde_cbor::Value>,
    gas_limit: u64,
    #[serde(with = "serde_bytes")]
    signature: Option<Vec<u8>>,
    network_id: u64,
    #[serde(with = "serde_bytes")]
    contract_address: Vec<u8>,
}

impl Transaction {
    pub fn run(
        vm_state: &mut vm::State,
        current_block: &Block,
        vm_transaction: vm::Transaction,
        position: i64,
    ) -> Self {
        let mut completed_transaction: models::Transaction = vm_transaction.run(vm_state).into();
        completed_transaction.block_hash = current_block.hash.clone();
        completed_transaction.set_hash();
        completed_transaction.position = position;
	println!("{} {} {:?}", completed_transaction.position, completed_transaction.function, serde_cbor::from_slice::<serde_cbor::Value>(&completed_transaction.return_value));
        insert_into(transactions_table)
            .values(&completed_transaction)
            .execute(&get_pg_connection())
            .unwrap();
        completed_transaction
    }

    pub fn set_hash(&mut self) {
        self.hash = sha256(to_vec(&TransactionWithoutHash::from(self.clone())).unwrap());
    }
}

impl From<Transaction> for TransactionWithoutHash {
    fn from(transaction: Transaction) -> Self {
        Self {
            arguments: from_slice(&transaction.arguments).unwrap(),
            contract_address: transaction.contract_address,
            nonce: transaction.nonce as u32,
            function: transaction.function,
            gas_used: transaction.gas_used as u64,
            position: transaction.position as u32,
            gas_limit: transaction.gas_limit as u64,
            network_id: transaction.network_id as u64,
            sender: transaction.sender,
            signature: Some(transaction.signature),
        }
    }
}

impl From<&vm::CompletedTransaction> for Transaction {
    fn from(transaction: &vm::CompletedTransaction) -> Self {
        Self {
            network_id: transaction.network_id as i64,
            hash: vec![],
            block_hash: vec![],
            contract_address: transaction.contract_address.clone(),
            position: 0,
            sender: transaction.sender.clone(),
            gas_limit: transaction.gas_limit as i64,
            gas_used: transaction.gas_used as i64,
            nonce: transaction.nonce as i64,
            function: transaction.function.clone(),
            arguments: to_vec(&transaction.arguments).unwrap(),
            return_value: to_vec(&transaction.return_value).unwrap(),
            signature: transaction.signature.clone().unwrap(),
        }
    }
}

impl From<&Transaction> for vm::Transaction {
    fn from(transaction: &Transaction) -> Self {
        Self {
            network_id: transaction.network_id as u32,
            contract_address: transaction.contract_address.clone(),
            sender: transaction.sender.clone(),
            gas_limit: transaction.gas_limit as u32,
            nonce: transaction.nonce as u32,
            function: transaction.function.clone(),
            arguments: from_slice(&transaction.arguments).unwrap(),
            signature: Some(transaction.signature.clone()),
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
            position: 0,
            sender: transaction.sender,
            gas_limit: transaction.gas_limit as i64,
            gas_used: transaction.gas_used as i64,
            nonce: transaction.nonce as i64,
            function: transaction.function,
            arguments: to_vec(&transaction.arguments).unwrap(),
            return_value: to_vec(&transaction.return_value).unwrap(),
            signature: transaction.signature.unwrap(),
        }
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
