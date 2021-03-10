use crate::config::HOST;
use crate::config::SIGNER;

use crate::config::GENESIS_NODE;
use crate::transaction::SignedSystemTransaction;
use crate::{config::OPTS, constants::DB, hash_onion, serde_cbor::Deserializer};
use ellipticoin_contracts::Action;
use ellipticoin_peerchain_ethereum::eth_address;
use std::fs::File;

pub async fn start_miner() {
    let mut backend = DB.get().unwrap().write().await;
    let store_lock = crate::db::StoreLock{guard: backend};
    let mut db = ellipticoin_types::Db {
backend: store_lock,
             transaction_state: Default::default(),
    };
    let start_mining_transaction = SignedSystemTransaction::new(
        &mut db,
        Action::StartMining(HOST.to_string(), hash_onion::peel().await),
    );
    start_mining_transaction.run(&mut db).await.unwrap();
    println!(
        "Started Miner: {}",
        hex::encode(eth_address(SIGNER.verify_key()))
    );
}
pub async fn catch_up() {
    if *GENESIS_NODE {
        return;
    }
    // let pg_db = get_pg_connection();
    // let mut won_blocks = 0;
    // for block_number in 0.. {
    //     if let Ok((block, transactions)) = get_block(block_number).await {
    //         if !block.sealed {
    //             break;
    //         }
    //
    //         // let state = block.apply(transactions).await;
    //         // if state.miners.first().unwrap().address == verification_key() {
    //         //     won_blocks += 1;
    //         // }
    //     } else {
    //         break;
    //     }
    // }
    // if won_blocks > 0 {
    //     HashOnion::skip(&pg_db, won_blocks);
    // }

    println!("Syncing complete");
}

pub async fn reset_state() {
    load_genesis_state().await;
    hash_onion::generate().await;
}

pub async fn load_genesis_state() {
    let mut backend = DB.get().unwrap().write().await;
    let store_lock = crate::db::StoreLock{guard: backend};
    let mut db = ellipticoin_types::Db {
backend: store_lock,
             transaction_state: Default::default(),
    };
    let genesis_file = File::open(OPTS.genesis_state_path.clone()).expect(&format!(
        "Genesis file {} not found",
        &OPTS.genesis_state_path
    ));

    for (key, value) in Deserializer::from_reader(&genesis_file)
        .into_iter::<(Vec<u8>, Vec<u8>)>()
        .map(Result::unwrap)
    {
        db.insert_raw(&key, &value);
        // ellipticoin_types::db::Backend::insert(db.backend, &key, &value);
    }
}
