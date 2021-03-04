use crate::config::HOST;
use crate::config::SIGNER;
use crate::db::MemoryDB;
use crate::transaction::SignedSystemTransaction;
use crate::{config::OPTS, hash_onion, serde_cbor::Deserializer, state::IN_MEMORY_STATE};
use ellipticoin_contracts::Action;
use ellipticoin_peerchain_ethereum::eth_address;
use std::fs::File;

pub async fn start_miner() {
    let mut state = IN_MEMORY_STATE.lock().await;
    let mut db = MemoryDB::new(&mut state);
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
    let mut state = IN_MEMORY_STATE.lock().await;
    let genesis_file = File::open(OPTS.genesis_state_path.clone()).expect(&format!(
        "Genesis file {} not found",
        &OPTS.genesis_state_path
    ));

    for (key, value) in Deserializer::from_reader(&genesis_file)
        .into_iter::<(Vec<u8>, Vec<u8>)>()
        .map(Result::unwrap)
    {
        state.insert(key, value);
    }
}
