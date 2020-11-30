use crate::{
    block_broadcaster::broadcast_block,
    constants::{NEW_BLOCK_CHANNEL, WEB_SOCKET_BROADCASTER},
};
pub use crate::{
    config::{get_pg_connection, verification_key},
    constants::TOKEN_CONTRACT,
    diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::bytes_to_value,
    models::{self, HashOnion, Transaction},
    schema::{blocks, blocks::dsl, transactions},
    system_contracts::ellipticoin::{self, Miner},
    transaction,
};
use diesel::dsl::{insert_into, max};
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

    pub async fn apply(&self, transactions: Vec<models::Transaction>) -> ellipticoin::State {
        insert_into(dsl::blocks)
            .values(self)
            .execute(&get_pg_connection())
            .unwrap();
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
        let state: ellipticoin::State =
            serde_cbor::from_slice::<Result<_, wasm_rpc::error::Error>>(
                &completed_transactions.last().unwrap().return_value,
            )
            .unwrap()
            .unwrap();
        WEB_SOCKET_BROADCASTER
            .broadcast(
                state.block_number as u32,
                state.miners.first().unwrap().host.clone(),
            )
            .await;
        state
    }

    pub fn insert(block_number: u32) -> Block {
        let block = Block {
            number: block_number as i32,
            ..Default::default()
        };
        insert_into(dsl::blocks)
            .values(&block)
            .execute(&get_pg_connection())
            .unwrap();
        block
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
            Transaction::run(&self, seal_transaction_request, transaction_position as i32).await;
        let state: ellipticoin::State =
            serde_cbor::from_slice::<Result<_, wasm_rpc::error::Error>>(
                &seal_transaction.return_value,
            )
            .unwrap()
            .unwrap();
        NEW_BLOCK_CHANNEL.0.send(state.clone()).await;
        self.sealed = true;
        diesel::update(dsl::blocks.filter(dsl::number.eq(self.number.clone())))
            .set(dsl::sealed.eq(true))
            .execute(&pg_db)
            .unwrap();
        let transactions = Transaction::belonging_to(&self)
            .order(transactions::dsl::position.asc())
            .load::<Transaction>(&pg_db)
            .unwrap();
        broadcast_block((self.clone(), transactions), state.clone().miners).await;
        WEB_SOCKET_BROADCASTER
            .broadcast(
                self.number as u32,
                state.miners.first().unwrap().host.clone(),
            )
            .await;
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
