use ellipticoind::{
    config::{SubCommand, OPTS},
<<<<<<< HEAD
    db,
    sub_commands::{self, generate_keypair},
};
use std::process;

#[async_std::main]
async fn main() {
    ctrlc::set_handler(move || {
        async_std::task::block_on(async {
            db::dump().await;
            process::exit(0)
        })
    })
    .expect("Error setting Ctrl-C handler");
    match &OPTS.subcmd {
        Some(SubCommand::GenerateKeypair) => generate_keypair(),
=======
    dump_v2_genesis,
    sub_commands::{self, dump_blocks, generate_keypair},
};

#[async_std::main]
async fn main() {
    match &OPTS.subcmd {
        Some(SubCommand::GenerateKeypair) => generate_keypair(),
        Some(SubCommand::DumpV2Genesis) => dump_v2_genesis::dump_v2_genesis().await,
        Some(SubCommand::DumpBlocks { block_number, file }) => {
            dump_blocks(*block_number, &file).await
        }
>>>>>>> master
        None => sub_commands::main().await,
    }
}
