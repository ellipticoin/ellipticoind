use crate::config::HOST;
use crate::config::SIGNER;
use crate::config::{self, address};
use crate::transaction::SignedSystemTransaction;
use crate::{config::OPTS, constants::DB, hash_onion, serde_cbor::Deserializer};
use ellipticoin_contracts::Action;
use ellipticoin_contracts::{Ellipticoin, Miner};
use ellipticoin_peerchain_ethereum::eth_address;
use ellipticoin_peerchain_ethereum::Signed;
use std::fs::File;
use std::path::Path;
use crate::db;

pub async fn start_miner() {
    let backend = DB.get().unwrap().write().await;
    let store_lock = crate::db::StoreLock { guard: backend };
    let mut db = ellipticoin_types::Db {
        backend: store_lock,
        transaction_state: Default::default(),
    };
    let start_mining_transaction = SignedSystemTransaction::new(
        &mut db,
        Action::StartMining(HOST.to_string(), hash_onion::peel().await),
    );
    if !Ellipticoin::get_miners(&mut db)
        .iter()
        .any(|Miner { address, .. }| address.clone() == config::address())
    {
        start_mining_transaction.run(&mut db).await.unwrap();
        println!(
            "Started Miner: {}",
            hex::encode(eth_address(SIGNER.verify_key()))
        );
    }
}
pub async fn catch_up() {
    println!("Syncing complete");
    if Path::new("transactions.cbor").exists() {
        let transacations_file = File::open("transactions.cbor").unwrap();
        for transaction in Deserializer::from_reader(&transacations_file)
            .into_iter::<SignedSystemTransaction>()
            .map(Result::unwrap)
        {
            let backend = DB.get().unwrap().write().await;
            let store_lock = crate::db::StoreLock { guard: backend };
            let mut db = ellipticoin_types::Db {
                backend: store_lock,
                transaction_state: Default::default(),
            };
            let result = transaction.apply(&mut db).await;
            if transaction.sender().unwrap_or(Default::default()) == address()
                && matches!(&transaction.0.action, Action::Seal(_))
                && result.is_ok()
            {
                hash_onion::peel().await;
            }
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
    }
}
