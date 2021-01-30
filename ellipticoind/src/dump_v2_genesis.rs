use crate::{
    diesel::{ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::sha256,
    models::{get_pg_connection, Block},
    schema::blocks::dsl as blocks_dsl,
    start_up,
    state::IN_MEMORY_STATE,
};

use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fs::File,
};

#[repr(u16)]
pub enum V2Contracts {
    Bridge,
    Ellipticoin,
    Exchange,
    Token,
}

const ELC: [u8; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

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
    let mut v2_genesis_state = HashMap::new();
    for (key, value) in state.iter() {
        match key.clone() {
            mut key
                if key.starts_with(
                    &[&sha256("Token".as_bytes().to_vec()).to_vec(), &vec![0][..]].concat(),
                ) =>
            {
                key.drain(..33);
                let (address, token): ([u8; 20], [u8; 20]) = if key.starts_with(b"EllipticoinELC") {
                    key.drain(..14);
                    let address = if key == b"Ellipticoin" {
                        ELC    
                    } else {
                        key[..20][..].try_into().unwrap()
                    };
                    println!("{}", base64::encode(&address));
                    (address, ELC)
                } else {
                    key.drain(..6);
                    (key[20..40][..].try_into().unwrap(), key[..20].try_into().unwrap())
                };
                // println!("key[0]: {}", key[0]);
                println!("address: {}", base64::encode(&address));
                println!("token: {}", base64::encode(&token));
                // let address = key.split_off(key.len()-32);
                // let token = key.clone();
                // // println!("{}", base64::encode(&adddres));
                // println!("token: {}", base64::encode(token_bytes_v2_token_bytes(&token)));
                // println!("address: {}", base64::encode(&address));
                //
                // // println!("{}", base64::encode(&key[key.len()-32.. key.len()]));
                println!("{}", 
                    base64::encode(v2_db_key(
                        V2Contracts::Token,
                        0u16,
                        &[&address[..], &token[..]].concat(),
                    )));

                v2_genesis_state.insert(
                    v2_db_key(
                        V2Contracts::Token,
                        0u16,
                        &[&address[..], &token[..]].concat(),
                    ),
                    value,
                );
            }
            _ => {
                println!("unknown key");
            }
        };
    }
    let file = File::create("/Users/masonf/tmp/genesis.cbor").unwrap();
    for (key, value) in v2_genesis_state.iter() {
        serde_cbor::to_writer(&file, &(key, value)).unwrap();
    }
}

fn v2_db_key(contract: V2Contracts, index: u16, key: &[u8]) -> Vec<u8> {
    [
        &(contract as u16).to_le_bytes()[..],
        &index.to_le_bytes().to_vec()[..],
        key,
    ]
    .concat()
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
