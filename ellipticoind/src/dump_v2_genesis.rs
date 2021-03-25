use crate::{
    diesel::{ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::sha256,
    models::{get_pg_connection, Block},
    schema::blocks::dsl as blocks_dsl,
    start_up,
    state::IN_MEMORY_STATE,
    models::{Transaction},
    schema::{transactions::dsl as transactions_dsl},
    legacy,
};
use std::convert::TryFrom;
use crate::system_contracts::api::InMemoryAPI;
use hex_literal::hex;
use std::{
    collections::HashMap,
    convert::{TryInto},
    fs::File,
};
use ellipticoin::Address;
use ellipticoin::Address::PublicKey;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;

#[repr(u16)]
pub enum V2Contracts {
    AMM,
    Bridge,
    Ellipticoin,
    Governance,
    OrderBook,
    System,
    Token,
}
struct V2Key(V2Contracts, u16, Vec<u8>);

const HACKER_ADDRESSES: [[u8; 32]; 2] = [
    hex!("b3fa7979614109d20b32da16854c57f803d62a4c66809790f25913714a831615"),
    hex!("1fb0c9ea9d1f0aa2a82afb7ccdebf0061b1aa0e05480538a777efbee77900a28"),
];
const V1_BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const V1_ETH: [u8; 20] = hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
const V2_BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const V2_ETH: [u8; 20] = hex!("0000000000000000000000000000000000000000");
const V2_ELC: [u8; 20] = hex!("0000000000000000000000000000000000000002");
const V1_USD: [u8; 20] = hex!("6b175474e89094c44da98b954eedeac495271d0f");
const V2_USD: [u8; 20] = hex!("5d3a536e4d6dbd6114cc1ead35777bab948e3643");
// const V2_USD: [u8; 20] = hex!("6d7f0754ffeb405d23c51ce938289d4835be3b14");
const USD_EXCHANGE_RATE: u128 = 211367456115200165329965416;

pub async fn dump_v2_genesis() {
    let pg_db = get_pg_connection();
    let blocks_query = blocks_dsl::blocks.into_boxed();
    let _blocks = blocks_query
        .order(blocks_dsl::number.asc())
        .load::<Block>(&pg_db)
        .unwrap();
    start_up::load_genesis_state().await;
    run_transactions_in_db().await;
    let state = IN_MEMORY_STATE.lock().await;
    let mut v2_genesis_state = HashMap::new();
    state
        .iter()
        .filter_map(|(key, value)| match key.clone() {
            mut key
                if key.starts_with(
                    &[&sha256("Token".as_bytes().to_vec()).to_vec(), &vec![0][..]].concat(),
                ) =>
            {
                key.drain(..33);
                let value = if is_usd(key.clone()) {
                    scale_usd_amount(value)
                } else {
                    value.to_vec()
                };
                if is_liquidity_token(&key) {
    if base64::encode(&convert_address_token_key(key.clone())[..20]) == "vQMn3JvS3ATITteQ+gOYfuVSn2Y=" {
    }
                Some((
                    V2Key(V2Contracts::AMM, 0, convert_address_token_key(key)),
                    value.clone(),
                ))
                } else {
                Some((
                    V2Key(V2Contracts::Token, 0, convert_address_token_key(key)),
                    value.clone(),
                ))
                }
            }
            mut key
                if key.starts_with(
                    &[&sha256("Token".as_bytes().to_vec()).to_vec(), &vec![1][..]].concat(),
                ) =>
            {
                key.drain(..33);
                let value = if is_usd(key.clone()) {
                    scale_usd_amount(value)
                } else {
                    value.to_vec()
                };
                if is_liquidity_token(&key) {
                Some((
                    V2Key(V2Contracts::AMM, 1, convert_token_key(key)),
                    value.clone(),
                ))
                } else {
                Some((
                    V2Key(V2Contracts::Token, 1, convert_token_key(key)),
                    value.clone(),
                ))
                }
            }
            mut key
                if key.starts_with(
                    &[
                        &sha256("Exchange".as_bytes().to_vec()).to_vec(),
                        &vec![0][..],
                    ]
                    .concat(),
                ) =>
            {
                key.drain(..33);
                Some((
                    V2Key(V2Contracts::AMM, 2, convert_token_key(key)),
                    scale_usd_amount(value)
                ))
            }
            mut key
                if key.starts_with(
                    &[
                        &sha256("Exchange".as_bytes().to_vec()).to_vec(),
                        &vec![1][..],
                    ]
                    .concat(),
                ) =>
            {
                key.drain(..33);
                Some((
                    V2Key(V2Contracts::AMM, 3, convert_token_key(key)),
                    value.clone(),
                ))
            }
            mut key
                if key.starts_with(
                    &[
                        &sha256("Exchange".as_bytes().to_vec()).to_vec(),
                        &vec![2][..],
                    ]
                    .concat(),
                ) =>
            {
                key.drain(..33);
                Some((
                    V2Key(V2Contracts::AMM, 4, convert_token_key(key)),
                    convert_liquidity_providers(value),
                ))
            }
            mut key
                if key.starts_with(
                    &[
                        &sha256("Ellipticoin".as_bytes().to_vec()).to_vec(),
                        &vec![0][..],
                    ]
                    .concat(),
                ) =>
            {
                key.drain(..33);
                Some((
                    V2Key(V2Contracts::System, 0, key),
                    value.clone(),
                ))
            }
            mut key
                if key.starts_with(
                    &[
                        &sha256("Ellipticoin".as_bytes().to_vec()).to_vec(),
                        &vec![1][..],
                    ]
                    .concat(),
                ) =>
            {
                key.drain(..33);
                Some((
                    V2Key(V2Contracts::Ellipticoin, 0, key[0..20].to_vec()),
                    value.clone(),
                ))
            }
            key
                if key.starts_with(
                    &[
                        &sha256("Ellipticoin".as_bytes().to_vec()).to_vec(),
                        &vec![2][..],
                    ]
                    .concat(),
                ) => None,
            key
                if key.starts_with(
                    &[
                        &sha256("Ellipticoin".as_bytes().to_vec()).to_vec(),
                        &vec![3][..],
                    ]
                    .concat(),
                ) => None,
            _ => {
                None
            },
        })
        .for_each(|(key, value)| {
            v2_genesis_state.insert(v2_db_key(key), value);
        });
    let file = File::create("/Users/masonf/tmp/genesis.cbor").unwrap();
    for (key, value) in v2_genesis_state.iter() {
        serde_cbor::to_writer(&file, &(key, value)).unwrap();
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
        if is_hacker_transction(&transaction) {
            continue
        }
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
}

fn is_hacker_transction(transaction: &Transaction) -> bool {
    let is_bridge_transaction_by_hacker = {
        if !["mint"].contains(&transaction.function.as_ref()) {
            return false
        }
        let arguments: Vec<serde_cbor::Value> = serde_cbor::from_slice(&transaction.arguments).unwrap();
        let address_bytes: serde_bytes::ByteBuf = serde_cbor::value::from_value(arguments[1].clone()).unwrap();
        let address = <[u8; 32]>::try_from(address_bytes.to_vec());
        if address.is_err() {
            println!("invalid address {}", hex::encode(address_bytes));
        }
        HACKER_ADDRESSES.contains(&address.unwrap_or([0; 32]))
    };
    HACKER_ADDRESSES.contains(&transaction.sender.clone()[..].try_into().unwrap()) || is_bridge_transaction_by_hacker

}



fn is_usd(mut key: Vec<u8>) -> bool {
   if key.starts_with(b"Bridge") {
        key.drain(..6);
        let (token, _address) = key.split_at(20);
        return token == V1_USD
    }
    false
}

fn is_liquidity_token(key: &[u8]) -> bool {
   key.starts_with(b"Exchange")
}

fn scale_usd_amount(value: &[u8]) ->  Vec<u8> {
    let amount: u64 = serde_cbor::from_slice(value).unwrap();
    let scaled_amount = ((BigInt::from(amount)* BigInt::from(10u128.pow(28))/USD_EXCHANGE_RATE)).to_u64().unwrap();
    serde_cbor::to_vec(&scaled_amount).unwrap()
}

fn convert_address_token_key(mut key: Vec<u8>) -> Vec<u8> {
    let (token, address): ([u8; 20], [u8; 20]) = if key.starts_with(b"EllipticoinELC") {
        key.drain(..14);
        let address = if key == b"Ellipticoin" {
            pad_left(vec![V2Contracts::Ellipticoin as u8], 20)
                .try_into()
                .unwrap()
        } else if key == b"Exchange" {
            pad_left(vec![V2Contracts::AMM as u8], 20)
                .try_into()
                .unwrap()
        } else {
            key[..20][..].try_into().unwrap()
        };
        (
            pad_left(vec![V2Contracts::Ellipticoin as u8], 20)
                .try_into()
                .unwrap(),
            address,
        )
    } else if key.starts_with(b"Exchange") {
        key.drain(..8);
        (
            convert_liquidity_token(&key[..32]),
            key[32..52][..].try_into().unwrap(),
        )
    } else if key.starts_with(b"Bridge") {
        key.drain(..6);
        let (token, address) = key.split_at(20);
        let address = if address == b"Exchange" {
            pad_left(vec![V2Contracts::AMM as u8], 20)
                .try_into()
                .unwrap()
        } else {
            address[..20].try_into().unwrap()
        };
        (v2_token(token.try_into().unwrap()), address)
    } else {
        panic!("unknown key")
    };
    // if base64::encode(address) == "vQMn3JvS3ATITteQ+gOYfuVSn2Y=" {
    //     println!("{} {}", base64::encode(address), hex::encode(token));
    // }
    [address, token].concat()
}

fn convert_token_key(key: Vec<u8>) -> Vec<u8> {
    if key == b"EllipticoinELC" {
        pad_left(vec![V2Contracts::Ellipticoin as u8], 20)
            .try_into()
            .unwrap()
    } else if key.starts_with(b"Exchange") {
        if sha256(["Bridge".as_bytes(), &V1_BTC[..]].concat()).to_vec() == key[8..].to_vec() {
                    V2_BTC.to_vec()
        } else if sha256(["Bridge".as_bytes(), &V1_ETH[..]].concat()).to_vec() == key[8..].to_vec()
        {
                    V2_ETH.to_vec()
        } else if sha256(["Bridge".as_bytes(), &V1_USD[..]].concat()).to_vec() == key[8..].to_vec()
        {
                    V2_USD.to_vec()
        } else if sha256(b"EllipticoinELC".to_vec()).to_vec() == key[8..].to_vec()
        {
                    V2_ELC.to_vec()
        } else {
            key[8..].to_vec()
        }
    } else if key.starts_with(b"Bridge") {
        if key[6..].to_vec() == V1_ETH {
            V2_ETH.to_vec()
        } else if key[6..].to_vec() == V1_BTC {
            V2_BTC.to_vec()
        } else {
            key[6..].to_vec()
        }
    } else {
        panic!("failed to convert token key")
    }
}

fn convert_liquidity_token(key: &[u8]) -> [u8; 20] {
    if sha256(["Bridge".as_bytes(), &V1_BTC[..]].concat()).to_vec() == key[..32].to_vec() {
                V2_BTC
    } else if sha256(["Bridge".as_bytes(), &V1_ETH[..]].concat()).to_vec() == key[..32].to_vec() {
                V2_ETH
    } else if sha256(b"EllipticoinELC".to_vec()).to_vec() == key[..32].to_vec() {
                V2_ELC
    } else {
        key[..20].try_into().unwrap()
    }
}

fn convert_liquidity_providers(v1_liquidity_providers: &[u8]) -> Vec<u8> {
    let liquidity_providers: Vec<Address> = serde_cbor::from_slice(v1_liquidity_providers).unwrap();
    serde_cbor::to_vec(&liquidity_providers.iter().map(|address| if let PublicKey(public_key) = address{
    public_key[..20].try_into().unwrap()
} else {
    panic!("")
}).collect::<Vec<[u8;20]>>()).unwrap()
}

fn v2_token(address: [u8; 20]) -> [u8; 20] {
    match address {
        V1_ETH => pad_left(vec![0u8], 20).try_into().unwrap(),
        V1_BTC => V2_BTC,
        V1_USD => V2_USD,
        address => address,
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
