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
    state::State,
    system_contracts::{self, api::NativeAPI},
    transaction,
};
use diesel::{
    insert_into,
    r2d2::{ConnectionManager, PooledConnection},
    OptionalExtension, PgConnection, QueryDsl,
};
use ellipticoin::Address;
use serde::{Deserialize, Serialize};
use serde_cbor::{from_slice, to_vec};
use std::{convert::TryInto, str};

impl From<Transaction> for transaction::Transaction {
    fn from(transaction: Transaction) -> transaction::Transaction {
        transaction::Transaction {
            network_id: transaction.network_id as u32,
            sender: transaction.sender[..].try_into().unwrap(),
            arguments: from_slice(&transaction.arguments).unwrap(),
            contract: transaction.contract,
            function: transaction.function,
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
    pub contract: String,
    pub sender: Vec<u8>,
    pub nonce: i64,
    pub function: String,
    pub arguments: Vec<u8>,
    pub return_value: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct TransactionWithoutHash {
    nonce: u32,
    sender: Vec<u8>,
    function: String,
    position: u32,
    arguments: Vec<serde_cbor::Value>,
    signature: Option<Vec<u8>>,
    network_id: u64,
    contract: String,
}

impl Transaction {
    pub fn run(
        state: &mut State,
        current_block: &Block,
        vm_transaction: transaction::Transaction,
        position: i64,
    ) -> Self {
            let mut api = NativeAPI {
                transaction: vm_transaction.clone(),
                contract: vm_transaction.clone().contract,
                state,
                caller: Address::PublicKey(vm_transaction.sender),
                sender: vm_transaction.sender.clone(),
            };
        let mut completed_transaction: models::Transaction = system_contracts::run(&mut api, vm_transaction).into();
        completed_transaction.block_hash = current_block.hash.clone();
        completed_transaction.position = position;
        completed_transaction.set_hash();
        insert_into(transactions_table)
            .values(&completed_transaction)
            .execute(&get_pg_connection())
            .unwrap();
        completed_transaction
    }

    pub fn set_hash(&mut self) {
        self.hash = sha256(to_vec(&TransactionWithoutHash::from(self.clone())).unwrap()).to_vec();
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
            signature: Some(transaction.signature),
        }
    }
}

impl From<&transaction::CompletedTransaction> for Transaction {
    fn from(transaction: &transaction::CompletedTransaction) -> Self {
        Self {
            network_id: transaction.network_id as i64,
            hash: vec![],
            block_hash: vec![],
            contract: transaction.contract.clone(),
            position: 0,
            sender: transaction.sender.to_vec(),
            nonce: transaction.nonce as i64,
            function: transaction.function.clone(),
            arguments: to_vec(&transaction.arguments).unwrap(),
            return_value: to_vec(&transaction.return_value).unwrap(),
            signature: transaction.signature.clone().unwrap(),
        }
    }
}

impl From<&Transaction> for transaction::Transaction {
    fn from(transaction: &Transaction) -> Self {
        Self {
            network_id: transaction.network_id as u32,
            contract: transaction.contract.clone(),
            sender: transaction.sender.clone()[..].try_into().unwrap(),
            nonce: transaction.nonce as u32,
            function: transaction.function.clone(),
            arguments: from_slice(&transaction.arguments).unwrap(),
            signature: Some(transaction.signature.clone()),
        }
    }
}

impl From<transaction::CompletedTransaction> for Transaction {
    fn from(transaction: transaction::CompletedTransaction) -> Self {
        Self {
            network_id: transaction.network_id as i64,
            hash: vec![],
            block_hash: vec![],
            contract: transaction.contract,
            position: 0,
            sender: transaction.sender.to_vec(),
            nonce: transaction.nonce as i64,
            function: transaction.function,
            arguments: to_vec(&transaction.arguments).unwrap(),
            return_value: to_vec(&transaction.return_value).unwrap(),
            signature: transaction.signature.unwrap_or(vec![]),
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
