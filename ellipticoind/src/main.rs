#[macro_use]
extern crate clap;
use std::net::IpAddr;

#[derive(Clap, Debug)]
struct Opts {
    #[clap(short = "p", long = "port", default_value = "3030")]
    port: u16,
    #[clap(short = "b", long = "bind", default_value = "127.0.0.1")]
    bind_address: String,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();
    let socket = (opts.bind_address.parse::<IpAddr>().unwrap(), opts.port).into();

    ellipticoind::run(socket).await
}
