use ellipticoind::{
    config::{SubCommand, OPTS},
    sub_commands::{self, dump_blocks, generate_keypair},
};

#[async_std::main]
async fn main() {
    match &OPTS.subcmd {
        Some(SubCommand::GenerateKeypair) => generate_keypair(),
        Some(SubCommand::DumpBlocks { block_number, file }) => {
            dump_blocks(*block_number, &file).await
        }
        None => sub_commands::main().await,
    }
}
