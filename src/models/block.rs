use crate::{
    config::get_pg_connection,
    constants::TOKEN_CONTRACT,
    diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::bytes_to_value,
    models::{self, HashOnion, Transaction},
    schema::{blocks, blocks::dsl, transactions},
    state::{State, MINERS},
    system_contracts::ellipticoin::Miner,
    transaction,
};
use diesel::{dsl::insert_into, sql_query};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Insertable, Clone, Debug, Serialize, Deserialize)]
#[primary_key(number)]
pub struct Block {
    pub number: i32,
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
    pub sealed: bool,
}

#[derive(Insertable, Clone, Default, Debug, Serialize, Deserialize)]
#[table_name = "blocks"]
pub struct NewBlock {
    pub memory_changeset_hash: Vec<u8>,
    pub storage_changeset_hash: Vec<u8>,
    pub sealed: bool,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            number: 0,
            memory_changeset_hash: vec![],
            storage_changeset_hash: vec![],
            sealed: false,
        }
    }
}

impl Block {
    pub fn new() -> Self {
        let block = Self {
            number: 0,
            memory_changeset_hash: vec![],
            storage_changeset_hash: vec![],
            sealed: false,
        };
        block
    }

    pub async fn apply(self, vm_state: &mut State, transactions: Vec<models::Transaction>) {
        // insert_into(dsl::blocks)
        //     .values(&self)
        //     .execute(&get_pg_connection()).unwrap();
        let number = insert_into(dsl::blocks)
            .values(&self)
            .returning(blocks::dsl::number)
            .get_result::<i32>(&get_pg_connection())
            .unwrap();
        sql_query(format!(
            "SELECT setval('blocks_number_seq', {}, true)",
            number
        ))
        .execute(&get_pg_connection())
        .unwrap();
        // .load(&connection)
        let completed_transactions: Vec<Transaction> = transactions
            .iter()
            .map(|transaction| {
                Transaction::run(
                    vm_state,
                    &self,
                    transaction::TransactionRequest::from(transaction),
                    transaction.position,
                )
            })
            .collect();
        *MINERS.lock().await =
            serde_cbor::from_slice::<Result<Vec<Miner>, wasm_rpc::error::Error>>(
                &completed_transactions.last().unwrap().return_value,
            )
            .unwrap()
            .unwrap();
        println!("Applied block #{}", self.number);
    }

    pub fn insert() -> Block {
        let new_block: NewBlock = Default::default();
        let number = insert_into(dsl::blocks)
            .values(&new_block)
            .returning(blocks::dsl::number)
            .get_result::<i32>(&get_pg_connection())
            .unwrap();
        Block {
            number,
            memory_changeset_hash: new_block.memory_changeset_hash,
            storage_changeset_hash: new_block.storage_changeset_hash,
            sealed: new_block.sealed,
        }
    }

    pub async fn is_valid(&self) -> bool {
        true
    }

    pub async fn seal(&self, vm_state: &mut State, transaction_position: i64) -> Vec<Transaction> {
        let pg_db = get_pg_connection();
        let skin = HashOnion::peel(&pg_db);
        let seal_transaction = transaction::TransactionRequest::new(
            TOKEN_CONTRACT.clone(),
            "seal",
            vec![bytes_to_value(skin.clone())],
        );
        let completed_transaction = Transaction::run(
            vm_state,
            &self,
            seal_transaction,
            transaction_position as i32,
        );
        *MINERS.lock().await =
            serde_cbor::from_slice::<Result<Vec<Miner>, wasm_rpc::error::Error>>(
                &completed_transaction.return_value,
            )
            .unwrap()
            .unwrap();
        diesel::update(dsl::blocks.filter(dsl::number.eq(self.number.clone())))
            .set(dsl::sealed.eq(true))
            .execute(&pg_db)
            .unwrap();
        Transaction::belonging_to(self)
            .order(transactions::dsl::position.asc())
            .load::<Transaction>(&pg_db)
            .unwrap()
    }
}
