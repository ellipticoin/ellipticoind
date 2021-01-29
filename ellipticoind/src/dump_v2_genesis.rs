use crate::state::IN_MEMORY_STATE;
use std::fs::File;
use crate::models::Block;
use crate::models::get_pg_connection;
use crate::start_up;
use crate::schema::blocks::dsl as blocks_dsl;
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use ellipticoin::helpers::db_key;
use crate::helpers::sha256;
use std::collections::HashMap;

#[repr(u16)]
pub enum V2Contracts {
    Token,
    Bridge,
    Exchange,
}

pub async fn dump_v2_genesis() {
    let pg_db = get_pg_connection();
    let blocks_query = blocks_dsl::blocks.into_boxed();
    let _blocks = blocks_query
        .order(blocks_dsl::number.asc())
        .load::<Block>(&pg_db)
        .unwrap();
    start_up::run_transactions_in_db().await;
    let state = IN_MEMORY_STATE.lock().await;
    let mut v2_genesis_state =  HashMap::new();
    for (key, value) in state.iter() {
        match key.clone() {
            mut key if key.starts_with(&sha256("Token".as_bytes().to_vec())) => {
                v2_genesis_state.insert(v2_db_key(V2Contracts::Token, key[0] as u16, &key[1..]), value);
            }
            _ => {
                println!("unknown key");
            }
        };
    }
    let file = File::create("genesis.cbor").unwrap();
    for (key, value) in v2_genesis_state.iter() {
        serde_cbor::to_writer(&file, &(key, value)).unwrap();
    };
}

fn v2_db_key(contract: V2Contracts, index: u16, key: &[u8]) -> Vec<u8> {
    [&(contract as u16).to_le_bytes()[..], &index.to_le_bytes().to_vec()[..], key].concat()
    
}
