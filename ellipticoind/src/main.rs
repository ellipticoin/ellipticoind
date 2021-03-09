use ellipticoind::{
    config::{SubCommand, OPTS},
    sub_commands::{self, generate_keypair},
};
use std::process;

#[async_std::main]
async fn main() {
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        process::exit(0)
    })
    .expect("Error setting Ctrl-C handler");
    match &OPTS.subcmd {
        Some(SubCommand::GenerateKeypair) => generate_keypair(),
        None => sub_commands::main().await,
    }
}
