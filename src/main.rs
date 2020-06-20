use ellipticoind::{
    config::{SubCommand, OPTS},
    sub_commands::{self, generate_keypair},
};

#[async_std::main]
async fn main() {
    match OPTS.subcmd {
        Some(SubCommand::GenerateKeypair) => generate_keypair(),
        None => sub_commands::main().await,
    }
}
