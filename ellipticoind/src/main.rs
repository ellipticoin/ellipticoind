use ellipticoind::{
    config::{SubCommand, OPTS},
    sub_commands::{self, dump_state, generate_keypair},
};

#[async_std::main]
async fn main() {
    match OPTS.subcmd {
        Some(SubCommand::GenerateKeypair) => generate_keypair(),
        Some(SubCommand::DumpState { block_number, .. }) => dump_state(block_number).await,
        None => sub_commands::main().await,
    }
}
