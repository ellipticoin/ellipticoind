<<<<<<< HEAD
use clap::Clap;
use dotenv::dotenv;
use ellipticoin_peerchain_ethereum::eth_address;
use ellipticoin_types::Address;
use k256::ecdsa::SigningKey;
use serde::{Deserialize, Deserializer};
use std::{
    convert::TryInto,
=======
use crate::pg;
use clap::Clap;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use dotenv::dotenv;
use ed25519_zebra::{SigningKey, VerificationKey};
use rand::seq::SliceRandom;
use serde::{Deserialize, Deserializer};
use std::{
    convert::TryFrom,
>>>>>>> master
    env,
    net::{IpAddr, SocketAddr},
};

#[derive(Clap, Debug)]
pub struct Opts {
    #[clap(short = 'a', long = "bind-address", default_value = "0.0.0.0")]
    pub bind_address: String,
<<<<<<< HEAD
=======
    #[clap(short = 'b', long = "bootnodes")]
    pub bootnodes: Option<String>,
>>>>>>> master
    #[clap(short = 'd', long = "database-url")]
    pub database_url: Option<String>,
    #[clap(short = 'n', long = "network-id", default_value = "3750925312")]
    pub network_id: u32,
    #[clap(short = 'p', long = "port", default_value = "80")]
    pub port: u16,
    #[clap(long = "rocksdb-path", default_value = "./ellipticoind/db")]
    pub rocksdb_path: String,
    #[clap(
        long = "genesis-path",
        default_value = "./ellipticoind/dist/genesis.cbor"
    )]
    pub genesis_state_path: String,
    #[clap(
        long = "genesis-blocks-path",
        default_value = "./ellipticoind/static/genesis-blocks.cbor"
    )]
    pub genesis_blocks_path: String,
    #[clap(long = "save-state")]
    pub save_state: bool,
    #[clap(subcommand)]
    pub subcmd: Option<SubCommand>,
    #[clap(long = "websocket-port", default_value = "81")]
    pub websocket_port: u16,
    #[clap(short = 'i', long = "insecure")]
    pub insecure: bool,
    #[clap(short = 's', long = "skip-genesis-blocks")]
    pub skip_genesis_blocks: bool,
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    #[clap(name = "generate-keypair")]
    GenerateKeypair,
<<<<<<< HEAD
=======
    #[clap(name = "dump-v2-genesis")]
    DumpV2Genesis,
    #[clap(name = "dump-blocks")]
    DumpBlocks {
        #[clap(long = "at-block")]
        block_number: Option<u32>,
        #[clap(long = "file", default_value = "genesis-blocks.cbor")]
        file: String,
    },
>>>>>>> master
}

lazy_static! {
    pub static ref OPTS: Opts = {
        dotenv().ok();
        Opts::parse()
    };
    pub static ref HOST: String = env::var("HOST").unwrap();
<<<<<<< HEAD
    pub static ref HASH_ONION_SIZE: usize = env::var("HASH_ONION_SIZE")
        .unwrap()
        .parse()
        .unwrap_or(7889400);
=======
>>>>>>> master
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
<<<<<<< HEAD
    pub static ref PRIVATE_KEY: [u8; 32] = {
        hex::decode(&env::var("PRIVATE_KEY").expect("PRIVATE_KEY not set"))
            .expect("Invalid PRIVATE_KEY")
            .try_into()
            .expect("Invalid PRIVATE_KEY")
    };
    pub static ref SIGNER: SigningKey =
        SigningKey::from_bytes(&*PRIVATE_KEY).expect("Invalid PRIVATE_KEY");
=======
    pub static ref PG_POOL: pg::Pool = {
        let manager = ConnectionManager::<PgConnection>::new(&database_url());
        Pool::new(manager).unwrap()
    };
>>>>>>> master
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

<<<<<<< HEAD
=======
pub fn bootnodes() -> Vec<Bootnode> {
    let path = OPTS
        .bootnodes
        .clone()
        .unwrap_or("./ellipticoind/dist/bootnodes.yaml".to_string());

    let string = std::fs::read_to_string(path).unwrap();
    serde_yaml::from_str(&string).unwrap()
}

pub fn database_url() -> String {
    OPTS.database_url
        .clone()
        .unwrap_or(env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
}

>>>>>>> master
pub fn socket() -> SocketAddr {
    (OPTS.bind_address.parse::<IpAddr>().unwrap(), OPTS.port).into()
}

<<<<<<< HEAD
pub fn address() -> Address {
    eth_address(SIGNER.verify_key())
}
pub fn verification_key() -> Address {
    Address([0; 20])
    // VerificationKey::from(&signing_key()).into()
=======
pub fn signing_key() -> SigningKey {
    SigningKey::try_from(
        <[u8; 32]>::try_from(
            &base64::decode(&env::var("PRIVATE_KEY").expect("PRIVATE_KEY not set")).unwrap()[..32],
        )
        .unwrap(),
    )
    .unwrap()
}

pub fn verification_key() -> [u8; 32] {
    VerificationKey::from(&signing_key()).into()
>>>>>>> master
}

pub fn network_id() -> u32 {
    if cfg!(test) {
        0
    } else {
        OPTS.network_id
    }
}

<<<<<<< HEAD
=======
pub fn random_bootnode() -> Bootnode {
    let mut rng = rand::thread_rng();
    (*bootnodes().choose(&mut rng).unwrap()).clone()
}

pub fn get_pg_connection() -> pg::Connection {
    PG_POOL.get().unwrap()
}

>>>>>>> master
pub fn host_uri(host: &str) -> String {
    if OPTS.insecure {
        format!("http://{}", host)
    } else {
        format!("https://{}", host)
    }
}

pub fn ethereum_balances_path() -> String {
    env::var("ETHEREUM_BALANCES_PATH")
        .unwrap_or("./ellipticoind/static/ethereum-balances-10054080.bin".to_string())
}
