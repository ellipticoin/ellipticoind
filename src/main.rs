extern crate clap;
use crate::clap::Clap;
use dotenv::dotenv;
use ed25519_dalek::Keypair;

use std::env;
use std::net::IpAddr;

#[derive(Clap, Debug)]
struct Opts {
    #[clap(short = "p", long = "port", default_value = "80")]
    port: u16,
    #[clap(short = "a", long = "bind-address", default_value = "0.0.0.0")]
    bind_address: String,
    #[clap(long = "websocket-port", default_value = "81")]
    websocket_port: u16,
    #[clap(long = "rocksdb-path", default_value = "./db")]
    rocksdb_path: String,
    #[clap(long = "redis-url", default_value = "redis://127.0.0.1")]
    redis_url: String,
    #[clap(short = "d", long = "database-url")]
    database_url: Option<String>,
    #[clap(short = "b", long = "bootnodes")]
    bootnodes: Option<String>,
    #[clap(subcommand)]
    subcmd: Option<SubCommand>,
}

#[derive(Clap, Debug)]
enum SubCommand {
    #[clap(name = "generate-keypair")]
    GenerateKeypair,
}

#[async_std::main]
async fn main() {
    let opts: Opts = Opts::parse();
    match opts.subcmd {
        Some(SubCommand::GenerateKeypair) => ellipticoind::generate_keypair(),
        None => {
            dotenv().ok();
            let database_url = opts
                .database_url
                .unwrap_or(env::var("DATABASE_URL").expect("DATABASE_URL must be set"));
            let private_key =
                Keypair::from_bytes(&base64::decode(&env::var("PRIVATE_KEY").unwrap()).unwrap())
                    .unwrap();
            let websocket_port = opts.websocket_port;
            let socket = (opts.bind_address.parse::<IpAddr>().unwrap(), opts.port).into();

            ellipticoind::run(
                database_url,
                &opts.rocksdb_path,
                &opts.redis_url,
                socket,
                websocket_port,
                private_key,
                ellipticoind::config::bootnodes(opts.bootnodes),
            )
            .await
        }
    }
}
