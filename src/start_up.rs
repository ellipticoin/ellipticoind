use crate::{
    api::{
        views,
        state::State,
    },
    config::{
        ethereum_balances_path, random_bootnode, Bootnode, BURN_PER_BLOCK, GENESIS_NODE, HOST, OPTS,
    },
    constants::{Namespace, GENESIS_ADDRESS, TOKEN_CONTRACT, TOKEN_WASM_PATH, GENESIS_STATE_PATH},
    helpers::bytes_to_value,
    models::HashOnion,
    pg,
    transaction_processor::apply_block,
    vm::{
        self,
        redis::{self, Commands},
        rocksdb,
        state::db_key,
        Transaction,
    },
    BEST_BLOCK,
};
pub use diesel_migrations::revert_latest_migration;
use indicatif::ProgressBar;

use serde_cbor::{value::from_value, Value};
use std::{
    convert::TryInto,
    fs::File,
    io::{BufRead, Read},
    ops::DerefMut,
    sync::Arc,
};

pub async fn start_miner(pg_db: &pg::Connection, redis: &mut redis::Connection) {
    let start_mining_transaction = Transaction::new(
        "start_mining".to_string(),
        vec![
            ((*HOST).clone().to_string().clone()).into(),
            (*BURN_PER_BLOCK).into(),
            bytes_to_value(HashOnion::peel(&pg_db)),
        ],
    );
    if !*GENESIS_NODE {
        post_transaction(&random_bootnode(), start_mining_transaction.clone()).await;
    }

    process_transaction(redis, start_mining_transaction);
}

fn process_transaction(redis: &mut redis::Connection, transaction: Transaction) {
    redis
        .rpush::<&str, Vec<u8>, ()>(
            "transactions::pending",
            serde_cbor::to_vec(&transaction).unwrap(),
        )
        .unwrap();
}

async fn post_transaction(bootnode: &Bootnode, transaction: Transaction) {
    let uri = format!("http://{}/transactions", bootnode.host);
    let _res = surf::post(uri)
        .body_bytes(serde_cbor::to_vec(&transaction).unwrap())
        .await
        .unwrap();
}

pub async fn catch_up(db_pool: pg::Pool, redis_pool: redis::Pool, vm_state: &mut vm::State) {
    for block_number in 0.. {
        let mut res = surf::get(format!(
            "http://{}/blocks/{}",
            random_bootnode().host,
            block_number
        ))
        .await
        .unwrap();
        if res.status() == 200 {
            let body_bytes = res.body_bytes().await.unwrap();
            let block_value = serde_cbor::from_slice::<Value>(&body_bytes).unwrap();
            let block_view: views::Block = from_value(block_value).unwrap();
            let (block, mut transactions) = block_view.into();
            transactions.iter_mut().for_each(|transaction| {
                transaction.set_hash();
                transaction.block_hash = block.hash.clone();
            });
            let mut ordered_transactions = transactions.clone();
            ordered_transactions.sort_by(|a, b| {
                if a.function == "start_mining" {
                    std::cmp::Ordering::Less
                } else if b.function == "start_mining" {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            apply_block(
                redis_pool.get().unwrap(),
                vm_state,
                block.clone(),
                ordered_transactions,
                db_pool.get().unwrap(),
            )
            .await;
            *BEST_BLOCK.lock().await = Some(block.clone());
            println!("Applied block #{}", &block.number);
        } else {
            println!("Syncing complete");
            break;
        }
    }
}

pub async fn reset_state(
    rocksdb: Arc<rocksdb::DB>,
    pg_db: &pg::Connection,
    redis_pool: redis::Pool,
) {
    reset_redis(&mut redis_pool.get().unwrap()).await;
    reset_pg(pg_db).await;
    reset_rocksdb(rocksdb.clone()).await;
    import_ethereum_balances(rocksdb.clone()).await;
    set_token_contract(rocksdb.clone());
    load_genesis_state(&mut redis_pool.get().unwrap(), rocksdb.clone());
    reset_current_miner(rocksdb.clone());
    HashOnion::generate(pg_db);
}

pub fn reset_current_miner(
    rocksdb: Arc<rocksdb::DB>
) {
    rocksdb
        .delete(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]),
        )
        .unwrap();
    rocksdb
        .put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::CurrentMiner as u8]),
            GENESIS_ADDRESS.to_vec(),
        )
        .unwrap();
}
pub fn load_genesis_state(
    redis: &mut redis::Connection,
    rocksdb: Arc<rocksdb::DB>
) {
    let genesis_file = File::open(GENESIS_STATE_PATH).unwrap();
    let state: State = serde_cbor::from_reader(genesis_file).unwrap();
    for (key, value) in state.memory {
        let _:() = redis.set(key, value).unwrap();
    }
    for (key, value) in state.storage {
        rocksdb.put(key, value).unwrap();
    }
}
pub fn set_token_contract(rocksdb: Arc<rocksdb::DB>) {
    let mut token_file = File::open(TOKEN_WASM_PATH).unwrap();
    let mut token_wasm = Vec::new();
    token_file.read_to_end(&mut token_wasm).unwrap();
    rocksdb
        .put(db_key(&TOKEN_CONTRACT, &vec![]), &token_wasm)
        .unwrap();
}

async fn reset_redis(redis: &mut redis::Connection) {
    let _: () = redis::cmd("FLUSHDB").query(redis.deref_mut()).unwrap();
}

async fn reset_pg(pg_db: &pg::Connection) {
    diesel_migrations::embed_migrations!();
    for _ in 0..4 {
        let _ = revert_latest_migration(pg_db);
    }
    embedded_migrations::run(pg_db).unwrap();
}

async fn reset_rocksdb(rocksdb: Arc<rocksdb::DB>) {
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
}

async fn import_ethereum_balances(rocksdb: Arc<rocksdb::DB>) {
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
