use crate::{
    api,
    config::{get_pg_connection, socket, ENABLE_MINER, GENESIS_NODE},
    constants::NEW_BLOCK_CHANNEL,
    diesel::{BelongingToDsl, ExpressionMethods, GroupedBy, QueryDsl, RunQueryDsl},
    miner,
    models::{verification_key, Block, Transaction},
    schema::{blocks::dsl as blocks_dsl, transactions::dsl as transactions_dsl},
    start_up,
    state::{get_state},
};
use async_std::task::spawn;
use ed25519_zebra::{SigningKey, VerificationKey};
use futures::future;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, fs::File, str};
use tide::listener::ListenInfo;

#[derive(Serialize, Deserialize)]
pub struct Genesis {
    pub memory: HashMap<Vec<u8>, Vec<u8>>,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}

pub fn generate_keypair() {
    let signing_key = SigningKey::new(thread_rng());
    let verification_key = VerificationKey::from(&signing_key);
    println!(
        "Verification Key (Address): {}",
        base64::encode(&<[u8; 32]>::try_from(verification_key).unwrap())
    );
    println!(
        "Full Private Key: {}",
        base64::encode(
            &[
                <[u8; 32]>::try_from(signing_key).unwrap(),
                <[u8; 32]>::try_from(verification_key).unwrap()
            ]
            .concat()
        )
    );
}

pub async fn dump_blocks(block_number: Option<u32>, file_name: &str) {
    let pg_db = get_pg_connection();
    let blocks_query = if let Some(block_number) = block_number {
        blocks_dsl::blocks
            .filter(blocks_dsl::number.le(block_number as i32))
            .into_boxed()
    } else {
        blocks_dsl::blocks.into_boxed()
    };
    let blocks = blocks_query
        .order(blocks_dsl::number.asc())
        .load::<Block>(&pg_db)
        .unwrap();
    let file = File::create(file_name.clone()).unwrap();
    let mut transactions = vec![];
    for blocks_chunk in blocks.chunks(u16::MAX as usize) {
        transactions.extend(
            Transaction::belonging_to(blocks_chunk)
                .order(transactions_dsl::position.asc())
                .load::<Transaction>(&pg_db)
                .unwrap()
                .grouped_by(&blocks_chunk),
        );
    }

    serde_cbor::to_writer(
        file,
        &blocks
            .into_iter()
            .zip(transactions)
            .collect::<Vec<(Block, Vec<Transaction>)>>(),
    )
    .unwrap();
}

pub async fn main() {
    start_up::reset_state().await;
    if !*GENESIS_NODE {
        start_up::catch_up().await;
    }
    NEW_BLOCK_CHANNEL.0.send(get_state().await).await;
    let api = api::API::new();
    spawn(
        api.app
            .listen_with(socket(), |info: ListenInfo| async move {
                println!("started listening on {}!", info.connection());
                println!("Address: {}", base64::encode(verification_key()));
                if *ENABLE_MINER {
                    start_up::start_miner().await;
                }
                Ok(())
            }),
    );
    spawn(miner::run());
    future::pending().await
}
