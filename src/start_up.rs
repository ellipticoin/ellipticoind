use crate::models::HashOnion;
use crate::{
    api::views,
    config::{Bootnode, BURN_PER_BLOCK, ENABLE_MINER, HOST, OPTS},
    constants::{Namespace, GENESIS_ADDRESS, TOKEN_CONTRACT},
    diesel::{ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::random,
    schema::hash_onion::dsl::*,
    transaction_processor::apply_block,
    vm::{
        self,
        redis::{self, Commands},
        rocksdb,
        state::db_key,
    },
    BEST_BLOCK,
};
use diesel::{
    dsl::*,
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
pub use diesel_migrations::revert_latest_migration;
use indicatif::ProgressBar;
use rand::Rng;
use serde_cbor::{to_vec, value::from_value, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    convert::TryInto,
    env,
    fs::File,
    io::{BufRead, Read},
    ops::DerefMut,
    sync::Arc,
};

pub async fn start_miner(
    _db: &std::sync::Arc<rocksdb::DB>,
    pg_db: &PooledConnection<ConnectionManager<PgConnection>>,
    public_key: ed25519_dalek::PublicKey,
    bootnodes: &Vec<Bootnode>,
) {
    if *ENABLE_MINER {
        let bootnode = bootnodes.get(0).unwrap();
        let skin: Vec<u8> = hash_onion
            .select(layer)
            .order(id.desc())
            .first(pg_db)
            .unwrap();
        let start_mining_transaction = vm::Transaction {
            network_id: OPTS.network_id,
            contract_address: TOKEN_CONTRACT.to_vec(),
            sender: public_key.to_bytes().to_vec(),
            nonce: random(),
            function: "start_mining".to_string(),
            arguments: vec![
                env::var("HOST").unwrap().into(),
                serde_cbor::Value::Integer(*BURN_PER_BLOCK as i128),
                skin.into_iter()
                    .map(|n| n.into())
                    .collect::<Vec<serde_cbor::Value>>()
                    .into(),
            ],
            gas_limit: 10000000,
        };
        sql_query(
            "delete from hash_onion where id in (
        select id from hash_onion order by id desc limit 1
    )",
        )
        .execute(pg_db)
        .unwrap();
        post_transaction(bootnode, start_mining_transaction).await;
    }
}

async fn post_transaction(bootnode: &Bootnode, transaction: vm::Transaction) {
    let uri = format!("http://{}/transactions", bootnode.host);
    let _res = surf::post(uri)
        .body_bytes(serde_cbor::to_vec(&transaction).unwrap())
        .await
        .unwrap();
}

pub async fn catch_up(
    db_pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    redis_pool: vm::r2d2_redis::r2d2::Pool<vm::r2d2_redis::RedisConnectionManager>,
    vm_state: &mut vm::State,
    bootnodes: &Vec<Bootnode>,
) {
    let bootnode = bootnodes.get(0).unwrap();
    for block_number in 0.. {
        let mut res = surf::get(format!("http://{}/blocks/{}", bootnode.host, block_number))
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

pub fn generate_hash_onion(db: &PooledConnection<ConnectionManager<PgConnection>>) {
    let hash_onion_size = env::var(&"HASH_ONION_SIZE")
        .map(|hash_onion_size| hash_onion_size.parse().unwrap())
        .unwrap_or(31 * 24 * 60 * 60);
    let sql_query_size = 65534;
    let center: Vec<u8> = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(32)
        .collect();
    let mut onion = vec![center];

    println!("Generating Hash Onion");
    let pb = ProgressBar::new(hash_onion_size);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
            .progress_chars("=> "),
    );
    for _ in (0..hash_onion_size).step_by(sql_query_size) {
        pb.inc(sql_query_size as u64);
        for _ in 1..(sql_query_size) {
            onion.push(sha256(onion.last().unwrap().to_vec()));
        }
        let values: Vec<HashOnion> = onion
            .iter()
            .map(|hash| HashOnion {
                layer: hash.to_vec(),
            })
            .collect();
        let query = insert_into(hash_onion).values(&values);
        query.execute(db).unwrap();
        onion = vec![onion.last().unwrap().to_vec()];
    }
    pb.finish();
}

pub fn sha256(value: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(value);
    hasher.result().to_vec()
}

pub async fn reset_redis(redis: &mut redis::Connection) {
    let _: () = redis::cmd("FLUSHALL").query(redis.deref_mut()).unwrap();
}

pub async fn reset_pg(pg_db: &PooledConnection<ConnectionManager<PgConnection>>) {
    diesel_migrations::embed_migrations!();
    for _ in 0..4 {
        let _ = revert_latest_migration(pg_db);
    }
    embedded_migrations::run(pg_db).unwrap();
}

pub async fn reset_rocksdb(rocksdb: Arc<rocksdb::DB>) {
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

pub async fn import_ethereum_balances(rocksdb: Arc<rocksdb::DB>) {
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
    let file = File::open("dist/ethereum-balances-10054080.bin").unwrap();

    let metadata = std::fs::metadata("dist/ethereum-balances-10054080.bin").unwrap();
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

pub async fn reset_state(
    rocksdb: Arc<rocksdb::DB>,
    pg_db: &PooledConnection<ConnectionManager<PgConnection>>,
    redis_pool: redis::Pool,
) {
    reset_redis(&mut redis_pool.get().unwrap()).await;
    reset_pg(pg_db).await;
    if !OPTS.save_state {
        reset_rocksdb(rocksdb.clone()).await;
    }
    import_ethereum_balances(rocksdb.clone()).await;

    rocksdb
        .put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::CurrentMiner as u8]),
            GENESIS_ADDRESS.to_vec(),
        )
        .unwrap();
    let mut token_file = File::open("./contracts/token/dist/token.wasm").unwrap();
    let mut token_wasm = Vec::new();
    token_file.read_to_end(&mut token_wasm).unwrap();
    rocksdb
        .put(db_key(&TOKEN_CONTRACT, &vec![]), &token_wasm)
        .unwrap();
    generate_hash_onion(pg_db);
    let skin: Vec<Value> = hash_onion
        .select(layer)
        .order(id.desc())
        .first::<Vec<u8>>(pg_db)
        .unwrap()
        .into_iter()
        .map(|n| n.into())
        .collect();
    let mut miners = BTreeMap::new();
    miners.insert(
        GENESIS_ADDRESS
            .to_vec()
            .into_iter()
            .map(Value::from)
            .collect::<Vec<Value>>(),
        (HOST.to_string(), *BURN_PER_BLOCK, skin.clone()),
    );

    rocksdb
        .put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]),
            to_vec(&miners).unwrap(),
        )
        .unwrap();
    sql_query(
        "delete from hash_onion where id in (
        select id from hash_onion order by id desc limit 1
    )",
    )
    .execute(pg_db)
    .unwrap();
    pub const RANDOM_SEED: [u8; 16] = hex!("da466bf1ce3c69dbef918817305cf989");
    rocksdb
        .put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::_RandomSeed as u8]),
            RANDOM_SEED.to_vec(),
        )
        .unwrap();
}
