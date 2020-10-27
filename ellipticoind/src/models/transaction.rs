use crate::{
    config::get_pg_connection,
    diesel::{ExpressionMethods, RunQueryDsl},
    models::block::Block,
    schema::{
        transactions,
        transactions::{
            columns::{nonce, sender},
            dsl::transactions as transactions_table,
        },
    },
    state::STATE,
    system_contracts::{self, api::NativeAPI},
    transaction::TransactionRequest,
};
use diesel::{insert_into, OptionalExtension, QueryDsl};
use serde::{Deserialize, Serialize};
use serde_cbor::from_slice;
use std::{convert::TryInto, str};

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    Associations,
    PartialEq,
    Clone,
    Default,
    Debug,
    Serialize,
    Deserialize,
)]
#[belongs_to(Block, foreign_key = "block_number")]
#[primary_key(id)]
pub struct Transaction {
    pub id: i32,
    pub network_id: i64,
    pub block_number: i32,
    pub position: i32,
    pub contract: String,
    pub sender: Vec<u8>,
    pub nonce: i32,
    pub function: String,
    pub arguments: Vec<u8>,
    pub return_value: Vec<u8>,
    pub raw: Vec<u8>,
}

#[derive(Insertable, Default)]
#[table_name = "transactions"]
pub struct NewTransaction {
    pub network_id: i64,
    pub block_number: i32,
    pub position: i32,
    pub contract: String,
    pub sender: Vec<u8>,
    pub nonce: i32,
    pub function: String,
    pub arguments: Vec<u8>,
    pub return_value: Vec<u8>,
    pub raw: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct TransactionWithoutHash {
    nonce: u32,
    sender: Vec<u8>,
    contract: String,
    function: String,
    position: u32,
    arguments: Vec<serde_cbor::Value>,
    network_id: u64,
}

impl Transaction {
    pub async fn run(
        current_block: &Block,
        vm_transaction: TransactionRequest,
        position: i32,
    ) -> Self {
        let mut state = STATE.lock().await;
        let mut api = NativeAPI {
            transaction: vm_transaction.clone(),
            state: &mut state,
        };
        let return_value = system_contracts::run(&mut api, vm_transaction.clone());
        Transaction::insert(vm_transaction, current_block, position, return_value)
    }

    pub fn insert(
        vm_transaction: TransactionRequest,
        current_block: &Block,
        position: i32,
        return_value: serde_cbor::Value,
    ) -> Self {
        let mut completed_transaction = NewTransaction {
            network_id: vm_transaction.network_id as i64,
            block_number: current_block.number,
            sender: vm_transaction.sender[..].try_into().unwrap(),
            arguments: serde_cbor::to_vec(&vm_transaction.arguments).unwrap(),
            contract: vm_transaction.contract,
            function: vm_transaction.function,
            nonce: vm_transaction.nonce as i32,
            return_value: serde_cbor::to_vec(&return_value).unwrap(),
            ..Default::default()
        };
        completed_transaction.position = position;
        let id = insert_into(transactions_table)
            .values(&completed_transaction)
            .returning(transactions::dsl::id)
            .get_result::<i32>(&get_pg_connection())
            .unwrap();
        Transaction {
            id,
            arguments: completed_transaction.arguments,
            block_number: completed_transaction.block_number,
            contract: completed_transaction.contract,
            function: completed_transaction.function,
            network_id: completed_transaction.network_id,
            nonce: completed_transaction.nonce,
            position: completed_transaction.position,
            raw: completed_transaction.raw,
            return_value: completed_transaction.return_value,
            sender: completed_transaction.sender,
        }
    }
}

impl From<Transaction> for TransactionWithoutHash {
    fn from(transaction: Transaction) -> Self {
        Self {
            arguments: from_slice(&transaction.arguments).unwrap(),
            contract: transaction.contract,
            nonce: transaction.nonce as u32,
            function: transaction.function,
            position: transaction.position as u32,
            network_id: transaction.network_id as u64,
            sender: transaction.sender,
        }
    }
}

impl From<&Transaction> for TransactionRequest {
    fn from(transaction: &Transaction) -> Self {
        Self {
            network_id: transaction.network_id as u32,
            contract: transaction.contract.clone(),
            sender: transaction.sender.clone()[..].try_into().unwrap(),
            nonce: transaction.nonce as u32,
            function: transaction.function.clone(),
            arguments: from_slice(&transaction.arguments).unwrap(),
        }
    }
}

pub fn next_nonce(address: Vec<u8>) -> u32 {
    let pg_db = get_pg_connection();
    transactions::dsl::transactions
        .order(nonce.desc())
        .filter(sender.eq(address))
        .select(nonce)
        .first(&pg_db)
        .optional()
        .unwrap()
        .unwrap_or(1) as u32
        + 1
}
