use crate::{
    api::state::VMState,
    config::{
        ethereum_balances_path, get_pg_connection, get_redis_connection, get_rocksdb,
        random_bootnode, BURN_PER_BLOCK, GENESIS_NODE, HOST, OPTS,
    },
    constants::{Namespace, GENESIS_STATE_PATH, TOKEN_CONTRACT, TOKEN_WASM_PATH},
    helpers::{bytes_to_value, get_block, post_transaction},
    models,
    models::{Block, HashOnion},
    vm::{
        redis::{self, Commands},
        rocksdb,
        state::db_key,
        State, Transaction,
    },
};
use diesel_migrations::revert_latest_migration;
use indicatif::ProgressBar;

use std::{
    convert::TryInto,
    fs::File,
    io::{BufRead, Read},
    ops::DerefMut,
};

pub async fn start_miner(vm_state: &mut State) {
    let pg_db = get_pg_connection();
    let start_mining_transaction = Transaction::new(
        TOKEN_CONTRACT.to_vec(),
        "start_mining",
        vec![
            ((*HOST).clone().to_string().clone()).into(),
            (*BURN_PER_BLOCK).into(),
            bytes_to_value(HashOnion::peel(&pg_db)),
        ],
    );
    if *GENESIS_NODE {
        let block = Block::insert(vm_state).await;
        models::Transaction::run(vm_state, &block, start_mining_transaction, 0);
        block.seal(vm_state, 1).await;
    } else {
        post_transaction(&random_bootnode(), start_mining_transaction.clone()).await;
    }
}

pub async fn catch_up(vm_state: &mut State) {
    for block_number in 0.. {
        if let Some(block) = get_block(&random_bootnode(), block_number).await {
            if !block.sealed {
                break;
            }

            let (block, transactions) = block.into();
            block.apply(vm_state, transactions);
        } else {
            break;
        }
    }
    println!("Syncing complete");
}

pub async fn reset_state() {
    let rocksdb = get_rocksdb();
    rocksdb
        .delete(db_key(&TOKEN_CONTRACT, &vec![Namespace::BlockNumber as u8]))
        .unwrap();
    let pg_db = get_pg_connection();
    reset_redis().await;
    reset_pg().await;
    reset_rocksdb().await;
    import_ethereum_balances().await;
    load_genesis_state().await;
    reset_current_miner().await;
    set_token_contract().await;
    HashOnion::generate(&pg_db);
}

pub async fn reset_current_miner() {
    let rocksdb = get_rocksdb();
    rocksdb
        .delete(db_key(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]))
        .unwrap();
}
pub async fn load_genesis_state() {
    let mut redis = get_redis_connection();
    let rocksdb = get_rocksdb();
    let genesis_file = File::open(GENESIS_STATE_PATH).unwrap();
    let state: VMState = serde_cbor::from_reader(genesis_file).unwrap();
    for (key, value) in state.memory {
        let _: () = redis.set(key, value).unwrap();
    }
    for (key, value) in state.storage {
        rocksdb.put(key, value).unwrap();
    }
}
pub async fn set_token_contract() {
    let rocksdb = get_rocksdb();
    let mut token_file = File::open(TOKEN_WASM_PATH).unwrap();
    let mut token_wasm = Vec::new();
    token_file.read_to_end(&mut token_wasm).unwrap();
    rocksdb
        .put(db_key(&TOKEN_CONTRACT, &vec![]), &token_wasm)
        .unwrap();
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
                &vec![Namespace::EthereumBalances as u8],
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
            &vec![Namespace::EthereumBalances as u8],
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
                            vec![Namespace::EthereumBalances as u8],
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
