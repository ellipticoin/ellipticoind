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
use std::convert::TryInto;
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
    start_up::load_genesis_state().await;
    start_up::run_transactions_in_db().await;
    let state = IN_MEMORY_STATE.lock().await;
    let mut v2_genesis_state =  HashMap::new();
    for (key, value) in state.iter() {
        match key.clone() {
            mut key if key.starts_with(&[&sha256("Token".as_bytes().to_vec()).to_vec(), &vec![0][..]].concat()) => {
                key.drain(..33);
                let (token, address) = if key.starts_with(b"EllipticoinELC") {
                    let mut elc_address = [0; 20];
                    elc_address[19] = 1; 
                    (elc_address, key[14..34].to_vec()) 
                } else {
                    (key[6..26].try_into().unwrap(), key[26..46].to_vec())
                    };
                // println!("token: {}", hex::encode(token));
                println!("address: {}", base64::encode(&address));
                // let address = key.split_off(key.len()-32);
                // let token = key.clone();
                // // println!("{}", base64::encode(&adddres));
                // println!("token: {}", base64::encode(token_bytes_v2_token_bytes(&token)));
                // println!("address: {}", base64::encode(&address));
                //
                // // println!("{}", base64::encode(&key[key.len()-32.. key.len()]));
                v2_genesis_state.insert(v2_db_key(V2Contracts::Token, key[0] as u16, &[&address[..], &token[..]].concat()), value);
            }
            _ => {
                println!("unknown key");
            }
        };
    }
    let file = File::create("/Users/masonf/tmp/genesis.cbor").unwrap();
    for (key, value) in v2_genesis_state.iter() {
        serde_cbor::to_writer(&file, &(key, value)).unwrap();
    };
}

fn v2_db_key(contract: V2Contracts, index: u16, key: &[u8]) -> Vec<u8> {
    [&(contract as u16).to_le_bytes()[..], &index.to_le_bytes().to_vec()[..], key].concat()
    
}


fn token_bytes_v2_token_bytes(token: &[u8]) -> [u8; 20] { 
    if token == b"EllipticoinELC" {
        [0; 20]
    } else {
        token[6..26].try_into().unwrap()
    }
    // println!("{}", base64::encode(token));
    // match std::str::from_utf8(token).unwrap().as_ref() {
    //     "EllipticoinELC" => [0; 20],
    //     a => panic!("{}", a)
    // }
}
