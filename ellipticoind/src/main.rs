#![feature(async_closure)]
extern crate clap;
use crate::clap::Clap;
use dotenv::dotenv;
use ed25519_dalek::Keypair;
use std::env;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};

#[derive(Clap, Debug)]
struct Opts {
    #[clap(short = "p", long = "port", default_value = "4460")]
    port: u16,
    #[clap(short = "a", long = "bind-addres", default_value = "0.0.0.0")]
    bind_address: String,
    #[clap(long = "api-port", default_value = "4461")]
    api_port: u16,
    #[clap(long = "api-bind-address", default_value = "0.0.0.0")]
    api_bind_address: String,
    #[clap(long = "websocket-port", default_value = "4462")]
    websocket_port: u16,
    #[clap(long = "websocket-bind-address", default_value = "0.0.0.0")]
    websocket_bind_address: String,
    #[clap(long = "rocksdb-path", default_value = "./db")]
    rocksdb_path: String,
    #[clap(long = "redis-url", default_value = "redis://127.0.0.1")]
    redis_url: String,
    #[clap(short = "d", long = "database-url")]
    database_url: Option<String>,
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
            let api_socket = (
                opts.api_bind_address.parse::<IpAddr>().unwrap(),
                opts.api_port,
            )
                .into();
            let websocket_socket = (
                opts.websocket_bind_address.parse::<IpAddr>().unwrap(),
                opts.websocket_port,
            )
                .into();
            let database_url = opts
                .database_url
                .unwrap_or(env::var("DATABASE_URL").expect("DATABASE_URL must be set"));
            let mut bootnodes_txt = String::from(include_str!("bootnodes.txt"));
            bootnodes_txt.pop();
            let bootnodes = bootnodes_txt
                .split("\n")
                .map(|bootnode| {
                    let mut parts = bootnode.splitn(2, "/");
                    (
                        parts
                            .next()
                            .unwrap()
                            .parse::<SocketAddrV4>()
                            .unwrap()
                            .into(),
                        base64::decode(&parts.next().unwrap()).unwrap(),
                    )
                })
                .collect::<Vec<(SocketAddr, Vec<u8>)>>();
            let private_key =
                Keypair::from_bytes(&base64::decode(&env::var("PRIVATE_KEY").unwrap()).unwrap())
                    .unwrap();
            let socket = (opts.bind_address.parse::<IpAddr>().unwrap(), opts.port).into();

            ellipticoind::run(
                api_socket,
                websocket_socket,
                database_url,
                &opts.rocksdb_path,
                &opts.redis_url,
                socket,
                private_key,
                bootnodes,
            )
            .await
        }
    }
}
