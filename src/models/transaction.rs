use crate::{
    config::get_pg_connection,
    diesel::{ExpressionMethods, RunQueryDsl},
    error::CONTRACT_NOT_FOUND,
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
    system_contracts::{self, api::NativeAPI, is_system_contract},
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
use std::convert::TryInto;
use std::str;

impl From<Transaction> for transaction::Transaction {
    fn from(transaction: Transaction) -> transaction::Transaction {
        transaction::Transaction {
            network_id: transaction.network_id as u32,
            sender: transaction.sender[..].try_into().unwrap(),
            arguments: from_slice(&transaction.arguments).unwrap(),
            contract_address: (
                transaction.contract_address[0..32].try_into().unwrap(),
                str::from_utf8(&transaction.contract_address[32..])
                    .unwrap()
                    .to_string(),
            ),
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
        state: &mut State,
        current_block: &Block,
        vm_transaction: transaction::Transaction,
        position: i64,
    ) -> Self {
        let mut completed_transaction: models::Transaction = if is_system_contract(&vm_transaction)
        {
            let mut api = NativeAPI {
                transaction: vm_transaction.clone(),
                address: vm_transaction.clone().contract_address,
                state,
                caller: Address::PublicKey(vm_transaction.sender),
                sender: vm_transaction.sender.clone(),
            };
            system_contracts::run(&mut api, vm_transaction)
        } else {
            // user contracts are disabled for launch
            return vm_transaction
                .complete(
                    (CONTRACT_NOT_FOUND.clone()).into(),
                    vm_transaction.gas_limit,
                )
                .into();
        }
        .into();
        completed_transaction.block_hash = current_block.hash.clone();
        completed_transaction.set_hash();
        completed_transaction.position = position;
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

impl From<&transaction::CompletedTransaction> for Transaction {
    fn from(transaction: &transaction::CompletedTransaction) -> Self {
        Self {
            network_id: transaction.network_id as i64,
            hash: vec![],
            block_hash: vec![],
            contract_address: [
                &transaction.contract_address.0[..],
                transaction.contract_address.1.as_bytes(),
            ]
            .concat(),
            position: 0,
            sender: transaction.sender.to_vec(),
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

impl From<&Transaction> for transaction::Transaction {
    fn from(transaction: &Transaction) -> Self {
        Self {
            network_id: transaction.network_id as u32,
            contract_address: (
                transaction.contract_address[0..32].try_into().unwrap(),
                str::from_utf8(&transaction.contract_address[32..])
                    .unwrap()
                    .to_string(),
            ),
            sender: transaction.sender.clone()[..].try_into().unwrap(),
            gas_limit: transaction.gas_limit as u32,
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
            contract_address: [
                &transaction.contract_address.0[..],
                transaction.contract_address.1.as_bytes(),
            ]
            .concat(),
            position: 0,
            sender: transaction.sender.to_vec(),
            gas_limit: transaction.gas_limit as i64,
            gas_used: transaction.gas_used as i64,
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
