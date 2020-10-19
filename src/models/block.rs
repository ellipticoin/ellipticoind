use crate::{
    block_broadcaster::broadcast_block,
    constants::{set_miners, MINERS},
};
pub use crate::{
    config::{get_pg_connection, verification_key},
    constants::TOKEN_CONTRACT,
    diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::bytes_to_value,
    models::{self, HashOnion, Transaction},
    schema::{blocks, blocks::dsl, transactions},
    state::State,
    system_contracts::ellipticoin::Miner,
    transaction,
};
use diesel::{
    dsl::{insert_into, max},
    sql_query,
};
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

    pub async fn increment_block_number(number: i32) {
        sql_query(format!(
            "SELECT setval('blocks_number_seq', {}, true)",
            number
        ))
        .execute(&get_pg_connection())
        .unwrap();
    }
    pub async fn apply(self, transactions: Vec<models::Transaction>) -> Miner {
        let number = insert_into(dsl::blocks)
            .values(&self)
            .returning(blocks::dsl::number)
            .get_result::<i32>(&get_pg_connection())
            .unwrap();
        Block::increment_block_number(number as i32).await;
        let mut completed_transactions: Vec<Transaction> = vec![];
        for transaction in transactions {
            completed_transactions.push(
                Transaction::run(
                    &self,
                    transaction::TransactionRequest::from(transaction.clone()),
                    transaction.position,
                )
                .await,
            );
        }
        let miners = serde_cbor::from_slice::<Result<Vec<Miner>, wasm_rpc::error::Error>>(
            &completed_transactions.last().unwrap().return_value,
        )
        .unwrap()
        .unwrap();
        *MINERS.lock().await = Some(miners.clone());
        println!("Applied block #{}", self.number);
        miners.first().unwrap().clone()
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

    pub async fn seal(mut self, transaction_position: i64) {
        let pg_db = get_pg_connection();
        let skin = HashOnion::peel(&pg_db);
        let seal_transaction_request = transaction::TransactionRequest::new(
            TOKEN_CONTRACT.clone(),
            "seal",
            vec![bytes_to_value(skin.clone())],
        );
        let seal_transaction =
            Transaction::run(&self, seal_transaction_request, transaction_position as i32);
        let miners = serde_cbor::from_slice::<Result<Vec<Miner>, wasm_rpc::error::Error>>(
            &seal_transaction.await.return_value,
        )
        .unwrap()
        .unwrap();
        set_miners(miners.clone()).await;
        self.sealed = true;
        diesel::update(dsl::blocks.filter(dsl::number.eq(self.number.clone())))
            .set(dsl::sealed.eq(true))
            .execute(&pg_db)
            .unwrap();
        let transactions = Transaction::belonging_to(&self)
            .order(transactions::dsl::position.asc())
            .load::<Transaction>(&pg_db)
            .unwrap();
        broadcast_block((self, transactions), miners.clone()).await;
    }

    pub fn current_block_number() -> u32 {
        let pg_db = get_pg_connection();
        blocks::dsl::blocks
            .select(max(blocks::dsl::number))
            .filter(blocks::dsl::sealed.eq(true))
            .first::<Option<i32>>(&pg_db)
            .unwrap()
            .map(|n: i32| n as u32)
            .unwrap_or(0)
    }
}
