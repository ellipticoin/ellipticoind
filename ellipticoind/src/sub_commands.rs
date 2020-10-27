use crate::{
    api,
    config::{
        get_pg_connection, get_redis_connection, get_rocksdb, socket, SubCommand, ENABLE_MINER,
        GENESIS_NODE, OPTS,
    },
    constants::TOKEN_CONTRACT,
    diesel::{BelongingToDsl, ExpressionMethods, GroupedBy, QueryDsl, RunQueryDsl},
    miner,
    models::{Block, Transaction},
    schema::{blocks::dsl as blocks_dsl, transactions::dsl as transactions_dsl},
    start_up,
    start_up::{load_genesis_state, reset_redis, reset_rocksdb},
    state::{db_key, Memory, State, Storage},
    system_contracts,
    system_contracts::api::NativeAPI,
};
use async_std::task::spawn;
use ed25519_zebra::{SigningKey, VerificationKey};
use ellipticoin::{PrivateKey, PublicKey};
use futures::future;
use r2d2_redis::redis::Commands;
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
        base64::encode(&<PublicKey>::try_from(verification_key).unwrap())
    );
    println!(
        "Full Private Key: {}",
        base64::encode(
            &[
                <PrivateKey>::try_from(signing_key).unwrap(),
                <PublicKey>::try_from(verification_key).unwrap()
            ]
            .concat()
        )
    );
}

pub async fn dump_state(block_number: Option<u32>) {
    let pg_db = get_pg_connection();
    let blocks = if let Some(block_number) = block_number {
        blocks_dsl::blocks
            .filter(blocks_dsl::number.le(block_number as i32))
            .order(blocks_dsl::number.asc())
            .load::<Block>(&pg_db)
            .unwrap()
    } else {
        blocks_dsl::blocks
            .order(blocks_dsl::number.asc())
            .load::<Block>(&pg_db)
            .unwrap()
    };
    reset_redis().await;
    reset_rocksdb().await;
    load_genesis_state().await;
    let transactions = Transaction::belonging_to(&blocks)
        .order(transactions_dsl::position.asc())
        .load::<Transaction>(&pg_db)
        .unwrap()
        .grouped_by(&blocks);
    let memory = Memory {
        redis: get_redis_connection(),
    };
    let storage = Storage {
        rocksdb: get_rocksdb(),
    };
    let mut state = State::new(memory, storage);
    blocks
        .into_iter()
        .zip(transactions)
        .for_each(|(block, transactions)| {
            transactions.iter().for_each(|transaction| {
                let mut api = NativeAPI {
                    transaction: transaction.clone().into(),
                    state: &mut state,
                };
                let res = crate::system_contracts::run(&mut api, transaction.into());
                println!("{:?}", res);
            });
            println!("Applied block #{}", block.number);
        });
    let mut redis = get_redis_connection();
    let redis_keys: Vec<Vec<u8>> = redis.keys("*").unwrap_or(vec![]);
    println!("Saving state..");
    let memory = redis_keys
        .iter()
        .map(|key| {
            let value = redis.get(key.to_vec()).unwrap();
            (key.clone(), value)
        })
        .collect::<HashMap<Vec<u8>, Vec<u8>>>();
    let rocksdb = get_rocksdb();
    let storage = rocksdb
        .iterator(rocksdb::IteratorMode::Start)
        .filter(|(key, _value)| {
            !key.starts_with(&db_key(
                &TOKEN_CONTRACT,
                &vec![system_contracts::ellipticoin::StorageNamespace::EthereumBalances as u8],
            ))
        })
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<HashMap<Vec<u8>, Vec<u8>>>();
    let genesis = Genesis { memory, storage };
    let genesis_file_name = match &OPTS.subcmd {
        Some(SubCommand::DumpState { file, .. }) => file,
        _ => panic!(),
    };
    let genesis_file = File::create(genesis_file_name).unwrap();
    serde_cbor::to_writer(genesis_file, &genesis).unwrap();
    println!("Saved to {}", genesis_file_name);
}

pub async fn main() {
    start_up::reset_state().await;

    if !*GENESIS_NODE {
        start_up::catch_up().await;
    }

    spawn(miner::run());

    let api = api::API::new();
    spawn(
        api.app
            .listen_with(socket(), |info: ListenInfo| async move {
                println!("started listening on {}!", info.connection());
                if *ENABLE_MINER {
                    start_up::start_miner().await;
                }
                Ok(())
            }),
    );
    future::pending().await
}
