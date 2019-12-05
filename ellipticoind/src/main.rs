#[macro_use]
extern crate clap;
use dotenv::dotenv;
use std::env;
use std::include_bytes;
use std::net::IpAddr;

#[derive(Clap, Debug)]
struct Opts {
    #[clap(short = "p", long = "port", default_value = "4460")]
    port: u16,
    #[clap(short = "b", long = "bind", default_value = "127.0.0.1")]
    bind_address: String,
    #[clap(long = "redis-url", default_value = "redis://127.0.0.1")]
    redis_url: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let opts: Opts = Opts::parse();
    let socket = (opts.bind_address.parse::<IpAddr>().unwrap(), opts.port).into();
    let system_contract = include_bytes!("wasm/ellipticoin_system_contract.wasm");

    ellipticoind::run(
        socket,
        &database_url,
        &opts.redis_url,
        system_contract.to_vec(),
    )
    .await
}
