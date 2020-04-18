use crate::models::HashOnion;
use indicatif::ProgressBar;
use serde_bytes::ByteBuf;
use serde_cbor::to_vec;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use vm::state::db_key;

use crate::constants::{GENISIS_ADRESS, TOKEN_CONTRACT};
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use crate::schema::hash_onion::dsl::*;
use diesel::dsl::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection;
use rand::Rng;
use sha2::{Digest, Sha256};

enum Namespace {
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

pub fn generate_hash_onion(db: &PooledConnection<ConnectionManager<PgConnection>>) -> Vec<u8> {
    let hash_onion_size = 1000;
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

    hex!("66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925").to_vec()
}

pub fn sha256(value: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(value);
    hasher.result().to_vec()
}

pub async fn initialize_rocks_db(
    path: &str,
    pg_db: &PooledConnection<ConnectionManager<PgConnection>>,
) -> vm::rocksdb::DB {
    if Path::new(path).exists() {
        let db = vm::rocksdb::DB::open_default(path).unwrap();

        let mut token_file = File::open("../token/dist/token.wasm").unwrap();
        let mut token_wasm = Vec::new();
        token_file.read_to_end(&mut token_wasm).unwrap();
        db.put(db_key(&TOKEN_CONTRACT, &vec![]), &token_wasm)
            .unwrap();
        db.put(
            db_key(&TOKEN_CONTRACT, &vec![Namespace::CurrentMiner as u8]),
            GENISIS_ADRESS.to_vec(),
        )
        .unwrap();
        let mut miners: HashMap<ByteBuf, (u64, ByteBuf)> = HashMap::new();
        let skin: Vec<u8> = hash_onion
            .select(layer)
            .order(id.desc())
            .first(pg_db)
            .unwrap();
        miners.insert(
            ByteBuf::from(GENISIS_ADRESS.to_vec()),
            (100 as u64, ByteBuf::from(skin)),
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
    } else {
        let db = vm::rocksdb::DB::open_default(path).unwrap();
        let mut file = File::open("dist/ethereum-balances-9858734.bin").unwrap();
        let metadata = std::fs::metadata("dist/ethereum-balances-9858734.bin").unwrap();
        let pb = ProgressBar::new(metadata.len() / 24);
        println!("Importing Ethereum Balances");
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
                .progress_chars("=> "),
        );
        let mut batch = rocksdb::WriteBatch::default();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        let mut i = 0;
        for chunk in buffer.chunks(24) {
            batch.put(
                db_key(
                    &TOKEN_CONTRACT,
                    &[ETHEREUM_BALANCE_ENUM.to_vec(), chunk[0..20].to_vec()].concat(),
                ), // [ETHEREUM_BALANCE_PREFIX.to_vec(), chunk[0..20].to_vec()].concat()
                chunk[20..24].to_vec(),
            );
            if i % 1000 == 0 {
                pb.inc(1000);
            }
            i += 1
        }
        pb.finish();
        println!("Writing Ethereum balances to storage...");

        db.write(batch).unwrap();
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
        db.put(
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

        db
    }
}
