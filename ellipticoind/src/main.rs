use ellipticoind::{
    config::{SubCommand, OPTS},
    sub_commands::{self, generate_keypair},
};
use std::process;
use ellipticoind::constants::DB;
use std::collections::hash_map::Iter;
use ellipticoin_types::db::{Db, Backend};

use async_std::sync::RwLockWriteGuard;

struct StoreLock<'a, T> {
    guard: RwLockWriteGuard<'a, T>,
}

// pub async fn lock<'a,  B: ellipticoin_types::db::Backend>() -> StoreLock<'a, Db<ellipticoind::db::Backend>> {
//     let db=  DB.get().unwrap().write().await;
//     StoreLock{guard: db}
// }




struct Cursor {
    iter: std::vec::IntoIter<(Vec<u8>, Vec<u8>)>,
}


#[async_std::main]
async fn main() {
    ctrlc::set_handler(move || {
        async_std::task::block_on(async {
            // let  mut db = DB.get().unwrap().write().await;
            let lock = ellipticoind::db::lock().await;
            for (key, value) in  lock.get_cursor() {
                println!("{} {}", base64::encode(key), base64::encode(value));
            } 
            // let db = lock::<ellipticoind::db::Backend>().await;
            // let lock = db.get_cursor();
            // for (key, value) in db.all() {
            //     println!("{} {}", base64::encode(key), base64::encode(value));
            // }
            println!("received Ctrl+C!");
            process::exit(0)
        })
    })
    .expect("Error setting Ctrl-C handler");
    match &OPTS.subcmd {
        Some(SubCommand::GenerateKeypair) => generate_keypair(),
        None => sub_commands::main().await,
    }
}
