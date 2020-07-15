extern crate clap;
use crate::{pg, types};

use clap::Clap;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use dotenv::dotenv;
use ed25519_dalek::{Keypair, SecretKey};
use r2d2_redis::RedisConnectionManager;
use rand::seq::SliceRandom;
use serde::{Deserialize, Deserializer};
use std::{
    env,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

#[derive(Clap, Debug)]
pub struct Opts {
    #[clap(short = "a", long = "bind-address", default_value = "0.0.0.0")]
    pub bind_address: String,
    #[clap(short = "b", long = "bootnodes")]
    pub bootnodes: Option<String>,
    #[clap(short = "d", long = "database-url")]
    pub database_url: Option<String>,
    #[clap(short = "n", long = "network-id", default_value = "3750925312")]
    pub network_id: u32,
    #[clap(short = "p", long = "port", default_value = "80")]
    pub port: u16,
    #[clap(long = "rocksdb-path", default_value = "./db")]
    pub rocksdb_path: String,
    #[clap(long = "save-state")]
    pub save_state: bool,
    #[clap(long = "redis-url", default_value = "redis://127.0.0.1")]
    pub redis_url: String,
    #[clap(subcommand)]
    pub subcmd: Option<SubCommand>,
    #[clap(long = "websocket-port", default_value = "81")]
    pub websocket_port: u16,
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    #[clap(name = "generate-keypair")]
    GenerateKeypair,
    #[clap(name = "dump-state")]
    DumpState {
        #[clap(long = "at-block")]
        block_number: Option<u32>,
        #[clap(long = "file", default_value = "genesis.cbor")]
        file: String,
    },
}

lazy_static! {
    pub static ref OPTS: Opts = {
        dotenv().ok();
        Opts::parse()
    };
    pub static ref HOST: String = env::var("HOST").unwrap();
    pub static ref GENESIS_NODE: bool = env::var("GENESIS_NODE").is_ok();
    pub static ref ENABLE_MINER: bool = env::var("ENABLE_MINER").is_ok();
    pub static ref BURN_PER_BLOCK: u64 = {
        if *ENABLE_MINER {
            env::var("BURN_PER_BLOCK")
                .expect("BURN_PER_BLOCK not set")
                .parse()
                .unwrap()
        } else {
            0
        }
    };
    pub static ref REDIS_POOL: types::redis::Pool = {
        let redis_manager = RedisConnectionManager::new(OPTS.redis_url.clone()).unwrap();
        r2d2_redis::r2d2::Pool::builder()
            .build(redis_manager)
            .unwrap()
    };
    pub static ref PG_POOL: pg::Pool = {
        let manager = ConnectionManager::<PgConnection>::new(&database_url());
        Pool::new(manager).unwrap()
    };
    pub static ref ROCKSDB: Arc<rocksdb::DB> =
        Arc::new(rocksdb::DB::open_default(&OPTS.rocksdb_path).unwrap());
}

#[derive(Deserialize, Debug, Clone)]
pub struct Bootnode {
    pub host: String,
    #[serde(deserialize_with = "decode_base64")]
    public_key: Vec<u8>,
}

pub fn decode_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| base64::decode(&string).map_err(|err| Error::custom(err.to_string())))
}

pub fn bootnodes() -> Vec<Bootnode> {
    let path = OPTS
        .bootnodes
        .clone()
        .unwrap_or("dist/bootnodes.yaml".to_string());
    let string = std::fs::read_to_string(path).unwrap();
    serde_yaml::from_str(&string).unwrap()
}

pub fn database_url() -> String {
    OPTS.database_url
        .clone()
        .unwrap_or(env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
}

pub fn socket() -> SocketAddr {
    (OPTS.bind_address.parse::<IpAddr>().unwrap(), OPTS.port).into()
}

pub fn keypair() -> Keypair {
    Keypair::from_bytes(
        &base64::decode(&env::var("PRIVATE_KEY").expect("PRIVATE_KEY not set")).unwrap(),
    )
    .unwrap()
}

pub fn public_key() -> [u8; 32] {
    keypair().public.to_bytes()
}

pub fn network_id() -> u32 {
    if cfg!(test) {
        0
    } else {
        OPTS.network_id
    }
}

pub fn secret_key() -> SecretKey {
    keypair().secret
}

pub fn random_bootnode() -> Bootnode {
    let mut rng = rand::thread_rng();
    (*bootnodes().choose(&mut rng).unwrap()).clone()
}

pub fn get_redis_connection() -> types::redis::Connection {
    REDIS_POOL.get().unwrap()
}

pub fn get_pg_connection() -> pg::Connection {
    PG_POOL.get().unwrap()
}

pub fn get_rocksdb() -> Arc<rocksdb::DB> {
    (*ROCKSDB).clone()
}
pub async fn websocket_socket() -> SocketAddr {
    let mut websocket_socket = socket().clone();
    websocket_socket.set_port(OPTS.websocket_port);
    websocket_socket
}

pub fn ethereum_balances_path() -> String {
    env::var("ETHEREUM_BALANCES_PATH").unwrap_or("dist/ethereum-balances-10054080.bin".to_string())
}
