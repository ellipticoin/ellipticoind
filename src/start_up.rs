use crate::{
    client::{get_block, post_transaction},
    config::{
        ethereum_balances_path, get_pg_connection, get_redis_connection, get_rocksdb,
        random_bootnode, BURN_PER_BLOCK, GENESIS_NODE, HOST, OPTS,
    },
    constants::TOKEN_CONTRACT,
    helpers::bytes_to_value,
    models,
    models::{Block, HashOnion},
    state::{db_key, Memory, State, Storage},
    system_contracts,
    transaction::TransactionRequest,
};
use diesel_migrations::revert_latest_migration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use indicatif::ProgressBar;
use r2d2_redis::redis::{self};
use std::{convert::TryInto, fs::File, io::BufRead, ops::DerefMut};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VMState {
    pub memory: HashMap<Vec<u8>, Vec<u8>>,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}

pub async fn start_miner(vm_state: &mut State) {
    let pg_db = get_pg_connection();
    let skin = HashOnion::peel(&pg_db);
    let start_mining_transaction = TransactionRequest::new(
        TOKEN_CONTRACT.clone(),
        "start_mining",
        vec![
            ((*HOST).clone().to_string().clone()).into(),
            (*BURN_PER_BLOCK).into(),
            bytes_to_value(skin),
        ],
    );
    if *GENESIS_NODE {
        let block = Block::insert();
        models::Transaction::run(vm_state, &block, start_mining_transaction, 0);

        block.seal(vm_state, 1).await;
        println!("Created genisis block");
    } else {
        post_transaction(start_mining_transaction).await;
    }
}

pub async fn catch_up(vm_state: &mut State) {
    for block_number in 1.. {
        if let Ok((block, transactions)) = get_block(block_number).await {
            if !block.sealed {
                break;
            }

            block.apply(vm_state, transactions).await;
        } else {
            break;
        }
    }
    println!("Syncing complete");
}

pub async fn reset_state() {
    let pg_db = get_pg_connection();
    reset_redis().await;
    reset_pg().await;
    reset_rocksdb().await;
    // import_ethereum_balances().await;
    load_genesis_state().await;
    HashOnion::generate(&pg_db);
}

pub async fn load_genesis_state() {
    let mut memory = Memory {
        redis: get_redis_connection(),
    };
    let mut storage = Storage {
        rocksdb: get_rocksdb(),
    };
    let genesis_file = File::open(OPTS.genesis_state_path.clone()).expect(&format!(
        "Genesis file {} not found",
        &OPTS.genesis_state_path
    ));
    let state: VMState = serde_cbor::from_reader(genesis_file).unwrap();
    for (key, value) in state.memory {
        let _: () = memory.set(&key, &value);
    }
    for (key, value) in state.storage {
        storage.set(&key, &value);
    }
}

pub async fn reset_redis() {
    let mut redis = get_redis_connection();
    let _: () = redis::cmd("FLUSHDB").query(redis.deref_mut()).unwrap();
}

async fn reset_pg() {
    let pg_db = get_pg_connection();
    diesel_migrations::embed_migrations!();
    for _ in 0..4 {
        let _ = revert_latest_migration(&pg_db);
    }
    embedded_migrations::run(&pg_db).unwrap();
}

pub async fn reset_rocksdb() {
    let rocksdb = get_rocksdb();
    if OPTS.save_state {
        return;
    }
    println!("Resetting RocksDB");
    rocksdb
        .iterator(rocksdb::IteratorMode::Start)
        .filter(|(key, _value)| {
            !key.starts_with(&db_key(
                &TOKEN_CONTRACT,
                &vec![system_contracts::ellipticoin::StorageNamespace::EthereumBalances as u8],
            ))
        })
        .for_each(|(key, _value)| {
            rocksdb.delete(key).unwrap();
        });
    println!("Reset RocksDB");
}

async fn import_ethereum_balances() {
    let rocksdb = get_rocksdb();
    if rocksdb
        .prefix_iterator(db_key(
            &TOKEN_CONTRACT,
            &vec![system_contracts::ellipticoin::StorageNamespace::EthereumBalances as u8],
        ))
        .next()
        .is_some()
    {
        println!("Ethereum Balances Already Imported");
        return;
    }
    let file = File::open(ethereum_balances_path()).unwrap();

    let metadata = std::fs::metadata(ethereum_balances_path()).unwrap();
    let pb = ProgressBar::new(metadata.len() / 24);
    println!("Importing Ethereum Balances");
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
            .progress_chars("=> "),
    );
    let mut batch = rocksdb::WriteBatch::default();
    const CAP: usize = 24 * 1000;
    let mut reader = std::io::BufReader::with_capacity(CAP, file);

    loop {
        let length = {
            let buffer = reader.fill_buf().unwrap();
            for chunk in buffer.chunks(24) {
                batch.put(
                    db_key(
                        &TOKEN_CONTRACT,
                        &[
                            vec![
                                system_contracts::ellipticoin::StorageNamespace::EthereumBalances
                                    as u8,
                            ],
                            chunk[0..20].to_vec(),
                        ]
                        .concat(),
                    ),
                    (u64::from_le_bytes(
                        [chunk[20..24].to_vec(), [0; 4].to_vec()].concat()[..]
                            .try_into()
                            .unwrap(),
                    ) * 10)
                        .to_le_bytes()
                        .to_vec(),
                );
            }
            pb.inc(1000);
            rocksdb.write(batch).unwrap();
            batch = rocksdb::WriteBatch::default();
            buffer.len()
        };
        if length == 0 {
            break;
        }
        reader.consume(length);
    }
    pb.finish();
}
