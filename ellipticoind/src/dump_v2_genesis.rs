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
use hex_literal::hex;

#[repr(u16)]
pub enum V2Contracts {
    Bridge,
    Ellipticoin,
    Exchange,
    System,
    Token,
}
struct V2Key(V2Contracts, u16, Vec<u8>);

const V1_ETH: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const V1_BTC: [u8; 20] = hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
const V2_BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");

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
    state.iter().map(|(key, value)| {
        match key.clone() {
            mut key
                if key.starts_with(
                    &[&sha256("Token".as_bytes().to_vec()).to_vec(), &vec![0][..]].concat(),
                ) =>
            {
                key.drain(..33);
                (V2Key(V2Contracts::Token, 0, convert_address_token_key(key)), value)
            }
            mut key
                if key.starts_with(
                    &[&sha256("Token".as_bytes().to_vec()).to_vec(), &vec![1][..]].concat(),
                ) =>
            {
                key.drain(..33);
                (V2Key(V2Contracts::Token, 0, convert_token_key(key)), value)
            }
            mut key
                if key.starts_with(
                    &[&sha256("Exchange".as_bytes().to_vec()).to_vec(), &vec![0][..]].concat(),
                ) =>
            {
                key.drain(..33);
                (V2Key(V2Contracts::Exchange, 0, convert_token_key(key)), value)
            }
            mut key
                if key.starts_with(
                    &[&sha256("Exchange".as_bytes().to_vec()).to_vec(), &vec![1][..]].concat(),
                ) =>
            {
                key.drain(..33);
                (V2Key(V2Contracts::Exchange, 1, convert_token_key(key)), value)
            }
            _ => {
                (V2Key(V2Contracts::Token, 0, vec![]), value)
            }
        }
    }).for_each(|(key,value)| {
                v2_genesis_state.insert(
                    v2_db_key(
                        key,
                    ),
                    value
                );
    });
    let file = File::create("/Users/masonf/tmp/genesis.cbor").unwrap();
    for (key, value) in v2_genesis_state.iter() {
        serde_cbor::to_writer(&file, &(key, value)).unwrap();
    }
}


fn convert_address_token_key(mut key: Vec<u8>) -> Vec<u8> {
    let (token, address): ([u8; 20], [u8; 20]) = if key.starts_with(b"EllipticoinELC") {
        key.drain(..14);
        let address = if key == b"Ellipticoin" {
            pad_left(vec![V2Contracts::Ellipticoin as u8], 20).try_into().unwrap()   
        } else if key == b"Exchange" {
            pad_left(vec![V2Contracts::Exchange as u8], 20).try_into().unwrap()   
        } else {
            key[..20][..].try_into().unwrap()
        };
        (pad_left(vec![V2Contracts::Ellipticoin as u8], 20).try_into().unwrap(), address)
    } else if key.starts_with(b"Exchange"){
        key.drain(..8);
        (key[..20].try_into().unwrap(), key[20..40][..].try_into().unwrap())
    } else if key.starts_with(b"Bridge"){
        key.drain(..6);
        let (token, address) = key.split_at(20);
        let address = if address == b"Exchange" {
            pad_left(vec![V2Contracts::Exchange as u8], 20).try_into().unwrap()
        } else {
            address[..20].try_into().unwrap()
        };
        (v2_token(token.try_into().unwrap()), address)
    } else {
        panic!("unknown key")
    };
    [address, token].concat()
}

fn convert_token_key(key: Vec<u8>) -> Vec<u8> {
    if key == b"EllipticoinELC" {
        pad_left(vec![V2Contracts::Ellipticoin as u8], 20).try_into().unwrap()
    } else if key.starts_with(b"Exchange") {
        if sha256(["Bridge".as_bytes(), &V1_BTC[..]].concat()).to_vec() == key[8..].to_vec() {
            println!("{}", hex::encode(
            sha256([pad_left(vec![V2Contracts::Bridge as u8], 20).try_into().unwrap(), V2_BTC].concat())[..20].to_vec()));
            sha256([pad_left(vec![V2Contracts::Bridge as u8], 20).try_into().unwrap(), V2_BTC].concat())[..20].to_vec()
        }else if sha256(["Bridge".as_bytes(), &V1_ETH[..]].concat()).to_vec() == key[8..].to_vec() {
            sha256([pad_left(vec![V2Contracts::Bridge as u8], 20).try_into().unwrap(), [0; 20].to_vec()].concat())[..20].to_vec()
        } else {
            key[8..].to_vec()
        }
    } else if key.starts_with(b"Bridge") {
        key[6..].to_vec()
    } else {
        panic!("failed to convert token key")
    }
}

fn v2_token(address: [u8; 20]) -> [u8; 20] {
    match address {
        V1_ETH => pad_left(vec![0u8], 20).try_into().unwrap(),
        V1_BTC => V2_BTC,
        address => address
    }
}
fn v2_db_key(key: V2Key) -> Vec<u8> {
    [
        &(key.0 as u16).to_le_bytes()[..],
        &key.1.to_le_bytes().to_vec()[..],
        &key.2,
    ]
    .concat()
}

pub fn pad_left(value: Vec<u8>, padding_size: usize) -> Vec<u8> {
    let mut new_vec = vec![0; padding_size - value.len()];

    new_vec.splice(new_vec.len()..new_vec.len(), value.iter().cloned());
    new_vec
}

// fn convert_total_supply(key: &[u8], value: &[u8]) -> (Vec<u8>, Vec<u8>) {
//
// }
