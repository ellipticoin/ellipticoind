use crate::{
    api,
    config::{
        get_pg_connection, get_redis_connection, get_rocksdb, socket, websocket_socket, SubCommand,
        ENABLE_MINER, GENESIS_NODE, OPTS,
    },
    constants::{Namespace, TOKEN_CONTRACT},
    diesel::{BelongingToDsl, ExpressionMethods, GroupedBy, QueryDsl, RunQueryDsl},
    models::{Block, Transaction},
    run_loop,
    schema::{blocks::dsl as blocks_dsl, transactions::dsl as transactions_dsl},
    start_up,
    start_up::{load_genesis_state, reset_redis, reset_rocksdb, set_token_contract},
    vm,
    vm::{redis::Commands, state::db_key},
    VM_STATE, WEB_SOCKET,
};
use async_std::task::spawn;
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File};

#[derive(Serialize, Deserialize)]
pub struct Genesis {
    pub memory: HashMap<Vec<u8>, Vec<u8>>,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}

pub fn generate_keypair() {
    let mut os_rng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut os_rng);
    let public_key = base64::encode(&keypair.public.to_bytes());
    let private_key = base64::encode(&keypair.to_bytes().to_vec());
    println!("Public Key (Address): {}", public_key);
    println!("Private Key: {}", private_key);
}

pub async fn dump_state(block_number: Option<u32>) {
    let pg_db = get_pg_connection();
    let blocks = if let Some(block_number) = block_number {
        blocks_dsl::blocks
            .filter(blocks_dsl::number.le(block_number as i64))
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
    set_token_contract().await;
    let transactions = Transaction::belonging_to(&blocks)
        .order(transactions_dsl::position.asc())
        .load::<Transaction>(&pg_db)
        .unwrap()
        .grouped_by(&blocks);
    let mut vm_state = VM_STATE.lock().await;
    blocks
        .into_iter()
        .zip(transactions)
        .for_each(|(block, transactions)| {
            transactions.iter().for_each(|transaction| {
                let res = vm::Transaction::from(transaction).run(&mut vm_state);
                println!("{:?}", res);
            });
            vm_state.commit();
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
                &vec![Namespace::EthereumBalances as u8],
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
    {
        let mut vm_state = VM_STATE.lock().await;
        if !*GENESIS_NODE {
            start_up::catch_up(&mut vm_state).await;
        }
        if *ENABLE_MINER {
            start_up::start_miner(&mut vm_state).await;
        }
    }
    let (api_receiver, api_state) = api::API::new();
    spawn(api_state.listen(socket()));
    spawn(
        (*WEB_SOCKET)
            .lock()
            .await
            .clone()
            .bind(websocket_socket().await),
    );
    run_loop::run(api_receiver).await
}
