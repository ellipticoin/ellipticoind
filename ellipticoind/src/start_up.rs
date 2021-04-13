use crate::{
<<<<<<< HEAD
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
=======
    client::{download, get_block},
    config::{get_pg_connection, verification_key, BURN_PER_BLOCK, GENESIS_NODE, HOST, OPTS},
    constants::TOKEN_CONTRACT,
    diesel::{ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::{bytes_to_value, run_transaction},
    legacy, models,
    models::{Block, HashOnion, Transaction},
    schema::{blocks::dsl as blocks_dsl, transactions::dsl as transactions_dsl},
    serde_cbor::Deserializer,
    state::{is_mining, IN_MEMORY_STATE},
    static_files::STATIC_FILES,
    system_contracts::api::InMemoryAPI,
    transaction::TransactionRequest,
};
use diesel::{
    delete,
    dsl::{exists, not},
    sql_query,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::Path};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VMState {
    pub memory: HashMap<Vec<u8>, Vec<u8>>,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}

pub async fn start_miner() {
    if is_mining().await {
        return;
    }
    let pg_db = get_pg_connection();
    let skin = HashOnion::peel(&pg_db);
    let start_mining_transaction = TransactionRequest::new(
        TOKEN_CONTRACT.clone(),
        "start_mining",
        vec![
            ((*HOST).clone().to_string().clone()).into(),
            (*BURN_PER_BLOCK).into(),
            bytes_to_value(skin),
        ],
    );
    if *GENESIS_NODE {
        let block = Block::insert(0);
        models::Transaction::run(&block, start_mining_transaction, 0).await;
        println!("Won block #0");
        block.seal(1).await;
    } else {
        run_transaction(start_mining_transaction).await;
    }
}

pub async fn catch_up() {
    let pg_db = get_pg_connection();
    let mut won_blocks = 0;
    for block_number in 0.. {
        if let Ok((block, transactions)) = get_block(block_number).await {
            if !block.sealed {
                break;
            }

            let state = block.apply(transactions).await;
            if state.miners.first().unwrap().address == verification_key() {
                won_blocks += 1;
            }
        } else {
            break;
        }
    }
    if won_blocks > 0 {
        HashOnion::skip(&pg_db, won_blocks);
    }

    println!("Syncing complete");
}

pub async fn download_static_files() {
    let static_dir = Path::new("ellipticoind/static");
    for (file_name, hash) in STATIC_FILES.iter() {
        if !static_dir.join(file_name).exists() {
            download(file_name, static_dir.join(file_name), *hash).await
        }
    }
}
pub async fn reset_state() {
    download_static_files().await;
    load_genesis_state().await;

    if OPTS.save_state {
        run_transactions_in_db().await;
    } else {
        reset_pg().await;
        HashOnion::generate().await;
    }
}

pub async fn load_genesis_state() {
    let mut state = IN_MEMORY_STATE.lock().await;
>>>>>>> master
    let genesis_file = File::open(OPTS.genesis_state_path.clone()).expect(&format!(
        "Genesis file {} not found",
        &OPTS.genesis_state_path
    ));

    for (key, value) in Deserializer::from_reader(&genesis_file)
        .into_iter::<(Vec<u8>, Vec<u8>)>()
        .map(Result::unwrap)
    {
<<<<<<< HEAD
        db.insert_raw(&key, &value);
        db.flush();
    }
}
=======
        state.insert(key, value);
    }
}

pub async fn run_transactions_in_db() {
    let pg_db = get_pg_connection();
    let transactions = transactions_dsl::transactions
        .order((
            transactions_dsl::block_number.asc(),
            transactions_dsl::position.asc(),
        ))
        .load::<Transaction>(&pg_db)
        .unwrap();
    let mut state = IN_MEMORY_STATE.lock().await;
    for mut transaction in transactions {
        let mut api = InMemoryAPI::new(&mut state, Some(transaction.clone().into()));
        legacy::run(&mut api, &mut transaction).await;
        if transaction.id % 10000 == 0 && transaction.id != 0 {
            println!(
                "Applied transactions #{}-#{}",
                transaction.id - 10000,
                transaction.id
            )
        };
    }
    delete(blocks_dsl::blocks)
        .filter(not(exists(
            transactions_dsl::transactions
                .select(transactions_dsl::block_number)
                .filter(transactions_dsl::block_number.eq(blocks_dsl::number)),
        )))
        .execute(&pg_db)
        .unwrap();
}

async fn reset_pg() {
    let pg_db = get_pg_connection();
    diesel_migrations::embed_migrations!();
    embedded_migrations::run(&pg_db).unwrap();
    sql_query("TRUNCATE blocks CASCADE")
        .execute(&pg_db)
        .unwrap();
    sql_query("TRUNCATE hash_onion").execute(&pg_db).unwrap();
}
>>>>>>> master
