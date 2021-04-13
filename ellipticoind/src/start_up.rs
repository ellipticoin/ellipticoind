use crate::{
    config::{self, address, SIGNER},
    db,
    transaction::{new_start_mining_transaction, run},
};

use crate::{
    aquire_db_write_lock, config::OPTS, constants::DB, hash_onion, serde_cbor::Deserializer,
    transaction::SignedTransaction,
};

use ellipticoin_contracts::Miner;
use ellipticoin_peerchain_ethereum::eth_address;
use ellipticoin_types::traits::Run;
use std::{fs::File, path::Path};

pub async fn start_miner() {
    let start_mining_transaction = new_start_mining_transaction().await;
    let miners = db::get_miners().await;
    if !miners
        .iter()
        .any(|Miner { address, .. }| address.clone() == config::address())
    {
        run(start_mining_transaction).await.unwrap();
        println!(
            "Started Miner: {}",
            hex::encode(eth_address(SIGNER.verify_key()))
        );
    }
}

pub async fn catch_up() {
    println!("Syncing complete");
    if Path::new("var/transactions.cbor").exists() {
        let transacations_file = File::open("var/transactions.cbor").unwrap();
        for transaction in Deserializer::from_reader(&transacations_file)
            .into_iter::<SignedTransaction>()
            .map(Result::unwrap)
        {
            let result = crate::transaction::apply(&transaction).await;
            // println!("{:?} {} {} {:?}", transaction, transaction.sender().unwrap_or(Default::default()) == address(), transaction.is_seal(), result);
            if transaction.sender().unwrap_or(Default::default()) == address()
                && transaction.is_seal()
                && result.is_ok()
            {
                hash_onion::peel().await;
            }
            let mut db = aquire_db_write_lock!();
            db.flush();
        }
        db::verify().await;
    }
}

pub async fn reset_state() {
    load_genesis_state().await;
    hash_onion::generate().await;
}

pub async fn load_genesis_state() {
    let backend = DB.get().unwrap().write().await;
    let store_lock = crate::db::StoreLock { guard: backend };
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
        db.flush();
    }
}
