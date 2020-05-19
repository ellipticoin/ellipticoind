use crate::models::HashOnion;
use crate::network::Message;
use futures_util::sink::SinkExt;
use indicatif::ProgressBar;
use rand::Rng;
use serde_cbor::{to_vec, Value};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufRead;
use std::io::Read;
use std::path::Path;

use crate::constants::{GENISIS_ADRESS, TOKEN_CONTRACT};
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use crate::schema::hash_onion::dsl::*;
use async_std::task;
use diesel::dsl::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection;
use futures::channel::mpsc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::env;
use std::net::SocketAddr;
use vm::state::db_key;
use vm::Commands;

pub enum Namespace {
    _Allowences,
    Balances,
    CurrentMiner,
    Miners,
    RandomSeed,
    EthereumBalances,
}
pub const GENISIS_ETHEREUM_ADRESS: [u8; 20] = hex!("Adfe2B5BeAc83382C047d977db1df977FD9a7e41");
pub const RANDOM_SEED: [u8; 16] = hex!("46c621ec8e2478445018fb92ba7cc555");
lazy_static! {
    pub static ref RANDOM_SEED_ENUM: Vec<u8> = vec![4];
    pub static ref ETHEREUM_BALANCE_ENUM: Vec<u8> = vec![5];
    pub static ref BALANCES_ENUM: Vec<u8> = vec![1];
    pub static ref CURRENT_MINER_ENUM: Vec<u8> = vec![2];
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    #[serde(with = "serde_bytes")]
    pub contract_address: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
    pub nonce: u64,
    pub gas_limit: u64,
    pub function: String,
    pub arguments: Vec<serde_cbor::Value>,
}
pub fn start_miner(
    db: &std::sync::Arc<rocksdb::DB>,
    pg_db: &PooledConnection<ConnectionManager<PgConnection>>,
    redis: &mut vm::Client,
    public_key: ed25519_dalek::PublicKey,
    network_sender: mpsc::Sender<Message>,
) {
    if env::var("ENABLE_MINER").is_ok() {
        let burn_per_block: i128 = env::var("BURN_PER_BLOCK")
            .expect("BURN_PER_BLOCK no set")
            .parse()
            .unwrap();
        let miners: BTreeMap<Vec<u8>, (u64, Vec<u8>)> = serde_cbor::from_slice(
            &db.get(db_key(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]))
                .unwrap_or(Some(vec![]))
                .unwrap_or(vec![]),
        )
        .unwrap_or(BTreeMap::new());
        task::block_on(async {
            let skin: Vec<u8> = hash_onion
                .select(layer)
                .order(id.desc())
                .first(pg_db)
                .unwrap();
            let start_mining_transaction = vm::Transaction {
                contract_address: TOKEN_CONTRACT.to_vec(),
                sender: public_key.to_bytes().to_vec(),
                nonce: random(),
                function: "start_mining".to_string(),
                arguments: vec![
                    serde_cbor::Value::Integer(burn_per_block),
                    skin
            .into_iter()
            .map(|n| n.into())
            .collect::<Vec<serde_cbor::Value>>().into()
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

            if env::var("GENISIS_NODE").is_ok() {
                process_transaction(start_mining_transaction, redis);
            } else {
                let current_burn_per_block =
                    miners.get(&public_key.as_bytes().to_vec());
                if current_burn_per_block.is_none() {
                    post_transaction(start_mining_transaction, network_sender).await;
                }
            }
        });
        // }
    }
}
fn random() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0, u32::max_value() as u64)
}

async fn post_transaction(transaction: vm::Transaction, mut network_sender: mpsc::Sender<Message>) {
    network_sender
        .send(Message::Transaction(transaction))
        .await
        .unwrap();
}

fn process_transaction(transaction: vm::Transaction, redis: &mut vm::Client) {
    redis
        .rpush::<&str, Vec<u8>, ()>(
            "transactions::pending",
            serde_cbor::to_vec(&transaction).unwrap(),
        )
        .unwrap();
}
pub async fn catch_up(
    db: &PooledConnection<ConnectionManager<PgConnection>>,
    con: &mut vm::Client,
    vm_state: &mut vm::State, bootnodes: &Vec<SocketAddr>) {
    let mut bootnode = bootnodes[0];
    bootnode.set_port(4461);
    for block_number in 0.. {
        let mut res = surf::get(format!("http://{}/blocks/{}", bootnode, block_number))
            .await
            .unwrap();
        if res.status() == 200 {
            let block_view: crate::api::views::Block = serde_cbor::value::from_value(
                serde_cbor::from_slice::<serde_cbor::Value>(&res.body_bytes().await.unwrap())
                    .unwrap(),
            )
            .unwrap();
            let (block, mut transactions) = block_view.into();
            transactions.iter_mut().for_each(|transaction| {
                transaction.set_hash();
                transaction.block_hash = block.hash.clone();
            });
            println!("applying: {}", transactions
                     .clone()
                     .iter()
                     .map(|t|
                          format!("{} {:?}", t.function.clone(),
                          serde_cbor::from_slice::<serde_cbor::Value>(&t.arguments).unwrap()
                          )

                     )
                     .collect::<Vec<String>>()
                     .join(", "));

            crate::transaction_processor::apply_block(con, vm_state, block.clone(), transactions.clone()).await;
            vm_state.commit();
            block.clone().insert(&db, transactions.clone());
            *crate::BEST_BLOCK.lock().await = Some(block.clone());
            println!("random seed {}", base64::encode(
            &vm_state.get_storage(&[[0;32].to_vec(), "Ellipticoin".as_bytes().to_vec()].concat(), &vec![Namespace::RandomSeed as u8])));
            println!("Applied block #{}", &block.number);
        } else {
            println!("Syncing complete");
            break;
        }
    }
}
pub fn generate_hash_onion(db: &PooledConnection<ConnectionManager<PgConnection>>) {
    let hash_onion_size = 65534;
    // let hash_onion_size = 100;
    let center: Vec<u8> = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(32)
        .collect();
    let mut onion = vec![center];
    let pb = ProgressBar::new(hash_onion_size);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
            .progress_chars("=> "),
    );
    let mut i = 0;
    for _ in 1..(hash_onion_size) {
        onion.push(sha256(onion.last().unwrap().to_vec()));
        if i % 1000 == 0 {
            pb.inc(1000);
        }
        i += 1
    }
    pb.finish();
    let values: Vec<HashOnion> = onion
        .iter()
        .map(|hash| HashOnion {
            layer: hash.to_vec(),
        })
        .collect();
    let query = insert_into(hash_onion).values(&values);
    query.execute(db).unwrap();
}

pub fn sha256(value: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(value);
    hasher.result().to_vec()
}

pub async fn initialize_rocks_db(
    path: &str,
    pg_db: &PooledConnection<ConnectionManager<PgConnection>>,
    redis: &mut vm::Client,
) -> vm::rocksdb::DB {
    if Path::new(path).exists() {
        vm::rocksdb::DB::open_default(path).unwrap()
    } else {
        let db = vm::rocksdb::DB::open_default(path).unwrap();
        // let file = File::open("dist/ethereum-balances-9858734.bin").unwrap();
        let file = File::open("dist/development-balances.bin").unwrap();
        let metadata = std::fs::metadata("dist/ethereum-balances-9858734.bin").unwrap();
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
                            &[ETHEREUM_BALANCE_ENUM.to_vec(), chunk[0..20].to_vec()].concat(),
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
                db.write(batch).unwrap();
                batch = rocksdb::WriteBatch::default();
                buffer.len()
            };
            if length == 0 {
                break;
            }
            reader.consume(length);
        }
        pb.finish();

        let genesis_balance = db
            .get(db_key(
                &TOKEN_CONTRACT,
                &[
                    vec![Namespace::EthereumBalances as u8],
                    GENISIS_ETHEREUM_ADRESS.to_vec(),
                ]
                .concat(),
            ))
            .unwrap()
            .unwrap();
        db.delete(db_key(
            &TOKEN_CONTRACT,
            &[
                vec![Namespace::EthereumBalances as u8],
                GENISIS_ETHEREUM_ADRESS.to_vec(),
            ]
            .concat(),
        ))
        .unwrap();
        redis
            .set::<_, _, ()>(
                db_key(
                    &TOKEN_CONTRACT,
                    &[vec![Namespace::Balances as u8], GENISIS_ADRESS.to_vec()].concat(),
                ),
                genesis_balance,
            )
            .unwrap();
        db.put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::RandomSeed as u8]),
            RANDOM_SEED.to_vec(),
        )
        .unwrap();
        db.put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::CurrentMiner as u8]),
            GENISIS_ADRESS.to_vec(),
        )
        .unwrap();
        let mut token_file = File::open("../token/dist/token.wasm").unwrap();
        let mut token_wasm = Vec::new();
        token_file.read_to_end(&mut token_wasm).unwrap();
        db.put(db_key(&TOKEN_CONTRACT, &vec![]), &token_wasm)
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
        let mut miners: BTreeMap<Vec<Value>, (u64, Vec<Value>)> = BTreeMap::new();
        miners.insert(
            GENISIS_ADRESS.to_vec()
            .into_iter()
            .map(|n| n.into())
            .collect(),
            (100 as u64, skin.clone()),
        );

        sql_query(
            "delete from hash_onion where id in (
        select id from hash_onion order by id desc limit 1
    )",
        )
        .execute(pg_db)
        .unwrap();
        db.put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]),
            to_vec(&miners).unwrap(),
        )
        .unwrap();
        db.put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::RandomSeed as u8]),
            RANDOM_SEED.to_vec(),
        )
        .unwrap();
        db
    }
}
