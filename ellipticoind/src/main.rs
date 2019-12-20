#![feature(async_closure)]
#[macro_use]
extern crate clap;
use dotenv::dotenv;
use network::{Keypair, Server};
use std::env;
use std::include_bytes;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};

#[derive(Clap, Debug)]
struct Opts {
    #[clap(short = "p", long = "port", default_value = "4460")]
    port: u16,
    #[clap(short = "a", long = "bind-addres", default_value = "127.0.0.1")]
    bind_address: String,
    #[clap(long = "api-port", default_value = "4461")]
    api_port: u16,
    #[clap(long = "api-bind-address", default_value = "127.0.0.1")]
    api_bind_address: String,
    #[clap(long = "websocket-port", default_value = "4462")]
    websocket_port: u16,
    #[clap(long = "websocket-bind-address", default_value = "127.0.0.1")]
    websocket_bind_address: String,
    #[clap(long = "rocksdb-path", default_value = "./db")]
    rocksdb_path: String,
    #[clap(long = "redis-url", default_value = "redis://127.0.0.1")]
    redis_url: String,
    #[clap(short = "db", long = "database-url")]
    database_url: Option<String>,
}

#[async_std::main]
async fn main() {
    dotenv().ok();
    let opts: Opts = Opts::parse();
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
    let system_contract = include_bytes!("wasm/token.wasm");
    let mut bootnodes_txt = String::from(include_str!("bootnodes.txt"));
    bootnodes_txt.pop();
    let bootnodes = bootnodes_txt
        .split("\n")
        .map(|bootnode| {
            println!("{}", bootnode);
            bootnode.parse::<SocketAddrV4>().unwrap().into()
        })
        .collect::<Vec<SocketAddr>>();
    let key = Keypair::generate_ed25519();
    let socket = (opts.bind_address.parse::<IpAddr>().unwrap(), opts.port).into();
    let network = Server::new(key, socket, bootnodes).await.unwrap();
    // loop {
    // use network::{Sink, Stream, StreamExt};
    // network.send(vec![1,2,3]).await.unwrap();
    // let message: &Vec<u8> = &network.next().await.unwrap();
    // println!("message: {:?}", message);
    // }

    ellipticoind::run(
        api_socket,
        websocket_socket,
        &database_url,
        &opts.rocksdb_path,
        &opts.redis_url,
        network,
        system_contract.to_vec(),
    )
    .await
}
