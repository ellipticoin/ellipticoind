use crate::system_contracts::api::InMemoryAPI;
use crate::{
    diesel::{ExpressionMethods, QueryDsl, RunQueryDsl},
    helpers::sha256,
    legacy,
    models::Transaction,
    models::{get_pg_connection, Block},
    schema::blocks::dsl as blocks_dsl,
    schema::transactions::dsl as transactions_dsl,
    start_up,
    state::IN_MEMORY_STATE,
};
use ellipticoin::Address;
use ellipticoin::Address::PublicKey;
use hex_literal::hex;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use std::convert::TryFrom;
use std::time::SystemTime;
use std::{collections::HashMap, convert::TryInto, fs::File};

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

const HACKER_ADDRESSES: [[u8; 32]; 6] = [
    // Flooded trades
    hex!("b3fa7979614109d20b32da16854c57f803d62a4c66809790f25913714a831615"),
    // Made  a 1  unit trade but seems cool
    // hex!("0824073c36fbeaa0f32e2ecdd94c789b55ac8e17cb0b5606e031314bcf0a4500"),

    // Made 1 unit trades
    hex!("1fb0c9ea9d1f0aa2a82afb7ccdebf0061b1aa0e05480538a777efbee77900a28"),
    hex!("1a6e325901190934dab08e306938d4221b12050df83fb966b8b9f4f8877f37be"),
    hex!("e3886b6c604a20c21a3a24b509ac658f3763d04e53276bf8580ef39a426b5fdf"),
    hex!("4288e32b510f7f204be44d4671d3284582874d5dc3bfb5a8a74154ea639f58f5"),
    hex!("7c7ca82a864a71810c27d1dc1df20af94ecef7b8d3a26b1172537d5be7584670"),
];

lazy_static! {
    static ref V2_HACKER_ADDRESSES: Vec<[u8; 20]> = HACKER_ADDRESSES
        .iter()
        .map(|address| <[u8; 20]>::try_from(address[..20].to_vec()).unwrap())
        .collect::<Vec<[u8; 20]>>();
}

const V1_BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const V1_ETH: [u8; 20] = hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
const V2_BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const V2_ETH: [u8; 20] = hex!("0000000000000000000000000000000000000000");
const V2_ELC: [u8; 20] = hex!("0000000000000000000000000000000000000002");
const V1_USD: [u8; 20] = hex!("6b175474e89094c44da98b954eedeac495271d0f");
const V2_USD: [u8; 20] = hex!("5d3a536e4d6dbd6114cc1ead35777bab948e3643");
// const V2_USD: [u8; 20] = hex!("6d7f0754ffeb405d23c51ce938289d4835be3b14");
const USD_EXCHANGE_RATE: u128 = 213393371019770706290539363;
const TOKEN_BALANCE_KEY: [u8; 4] = [6, 0, 0, 0];
const BLOCK_NUMBER_KEY: [u8; 4] = [5, 0, 0, 0];
const POOL_SUPPLY_OF_BASE_TOKEN_KEY: [u8; 4] = [0, 0, 2, 0];
const POOL_SUPPLY_OF_TOKEN_KEY: [u8; 4] = [0, 0, 3, 0];
const TOTAL_SUPPLY_KEY: [u8; 4] = [6, 0, 3, 0];
const BASE_FACTOR: u64 = 1000000;
const BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const ETH: [u8; 20] = hex!("0000000000000000000000000000000000000000");
const MS: [u8; 20] = hex!("0000000000000000000000000000000000000002");
const USD: [u8; 20] = hex!("5d3a536e4d6dbd6114cc1ead35777bab948e3643");
// It looks as though ETH has been being stolen since the network launched
// This is the amount that was left after the hack
// https://etherscan.io/tx/0xbb3e03c5cb6804bf5d19167123285c23518dd06a47ac3de84e46e69296045265
const ETH_TOTAL_SUPPLY_AFTER_HACK: u64 = 4063245;

// 14K DAI was stolen out of the bridge using feeless 1 unit transactions
// https://etherscan.io/tx/0x91baf78c28ff576607a6723e10b164161e9f72ad38863460150d9d275ea25ecb
// 0.4175095-(14504.171817/59291.90) = 0.172886
const BTC_TOTAL_SUPPLY_AFTER_HACK: u64 = 172886;

const BTC_PRICE: u64 = 2952135284190;
const ETH_PRICE: u64 = 105390808967;
const TIME_OF_HACK: u64 = 1615754168;
const BASE_DAO_SEED_AMOUNT: u64 = 600000000000;

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
                    //                 println!("{}", hex::encode(v2_db_key(
                    //
                    //
                    //                     V2Key(V2Contracts::Token, 3, convert_token_key(key.clone())),
                    //
                    // )));
                    Some((
                        V2Key(V2Contracts::Token, 3, convert_token_key(key)),
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
                    scale_usd_amount(value),
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
                Some((V2Key(V2Contracts::System, 0, key), value.clone()))
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
            key if key.starts_with(
                &[
                    &sha256("Ellipticoin".as_bytes().to_vec()).to_vec(),
                    &vec![2][..],
                ]
                .concat(),
            ) =>
            {
                None
            }
            key if key.starts_with(
                &[
                    &sha256("Ellipticoin".as_bytes().to_vec()).to_vec(),
                    &vec![3][..],
                ]
                .concat(),
            ) =>
            {
                None
            }
            _ => None,
        })
        .for_each(|(key, value)| {
            v2_genesis_state.insert(v2_db_key(key), value);
        });
    let file = File::create("/Users/masonf/tmp/genesis.cbor").unwrap();
    remove_stolen_funds(&mut v2_genesis_state, ETH, ETH_TOTAL_SUPPLY_AFTER_HACK);
    remove_stolen_funds(&mut v2_genesis_state, BTC, BTC_TOTAL_SUPPLY_AFTER_HACK);
    strip_unknown_balances(&mut v2_genesis_state);
    let dao_address: [u8; 20] = pad_left(vec![V2Contracts::Governance as u8], 20)
        .try_into()
        .unwrap();
    let now: u64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let blocks_since_hack = (now - TIME_OF_HACK) / 4;
    let additional_seed_amount = blocks_since_hack * 1280000;
    let dao_seed_amount = BASE_DAO_SEED_AMOUNT + additional_seed_amount;
    fix_price(&mut v2_genesis_state, BTC, BTC_PRICE);
    fix_price(&mut v2_genesis_state, ETH, ETH_PRICE);
    fix_total_supply(&mut v2_genesis_state, MS);
    fix_total_supply(&mut v2_genesis_state, USD);
    credit(
        &mut v2_genesis_state,
        dao_address,
        dao_seed_amount as i64,
        MS,
    );
    fast_forward_block_number(&mut v2_genesis_state, blocks_since_hack);

    for (key, value) in v2_genesis_state.iter() {
        serde_cbor::to_writer(&file, &(key, value)).unwrap();
    }
}

fn fast_forward_block_number(state: &mut HashMap<Vec<u8>, Vec<u8>>, number_of_blocks: u64) {
    let block_number =
        serde_cbor::from_slice::<u64>(&state.get(&BLOCK_NUMBER_KEY.to_vec()).unwrap()).unwrap();
    *state.get_mut(&BLOCK_NUMBER_KEY.to_vec()).unwrap() =
        serde_cbor::to_vec(&(block_number + number_of_blocks)).unwrap();
}

fn fix_price(state: &mut HashMap<Vec<u8>, Vec<u8>>, token: [u8; 20], price: u64) {
    let amm_address: [u8; 20] = pad_left(vec![V2Contracts::AMM as u8], 20)
        .try_into()
        .unwrap();
    let pool_supply_of_token_key = [POOL_SUPPLY_OF_TOKEN_KEY.to_vec(), token.to_vec()].concat();
    let pool_supply_of_base_token_key =
        [POOL_SUPPLY_OF_BASE_TOKEN_KEY.to_vec(), token.to_vec()].concat();
    let pool_supply_of_token =
        serde_cbor::from_slice::<u64>(state.get(&pool_supply_of_token_key).unwrap()).unwrap();
    let pool_supply_of_base_token =
        serde_cbor::from_slice::<u64>(state.get(&pool_supply_of_base_token_key).unwrap()).unwrap();
    let token_off_by = (((pool_supply_of_base_token as i128 * BASE_FACTOR as i128)
        - (price as i128 * pool_supply_of_token as i128))
        / price as i128) as i64;
    let new_pool_supply_of_token =
        u64::try_from(pool_supply_of_token as i64 + token_off_by).unwrap();

    credit(state, amm_address, token_off_by, token);
    *state.get_mut(&pool_supply_of_token_key).unwrap() =
        serde_cbor::to_vec(&(new_pool_supply_of_token)).unwrap();
}

fn credit(state: &mut HashMap<Vec<u8>, Vec<u8>>, address: [u8; 20], amount: i64, token: [u8; 20]) {
    let balance_key = [TOKEN_BALANCE_KEY.to_vec(), address.to_vec(), token.to_vec()].concat();
    let balance =
        serde_cbor::from_slice::<u64>(state.get(&balance_key).unwrap_or(&vec![])).unwrap_or(0);
    *state
        .entry(balance_key.clone())
        .or_insert(Default::default()) =
        serde_cbor::to_vec(&((balance as i64 + amount) as u64)).unwrap();
    let total_supply_key = [TOTAL_SUPPLY_KEY.to_vec(), token.to_vec()].concat();
    let total_supply =
        serde_cbor::from_slice::<u64>(state.get(&total_supply_key).unwrap_or(&vec![])).unwrap_or(0);
    *state
        .entry(total_supply_key.clone())
        .or_insert(Default::default()) =
        serde_cbor::to_vec(&((total_supply as i64 + amount) as u64)).unwrap();
}

fn fix_total_supply(state: &mut HashMap<Vec<u8>, Vec<u8>>, token: [u8; 20]) {
    let token_balance_keys = state
        .keys()
        .cloned()
        .filter(|key| key.starts_with(&TOKEN_BALANCE_KEY.to_vec()) && key[24..] == token)
        .collect::<Vec<Vec<u8>>>();

    let mut new_total_supply = 0;
    for key in &token_balance_keys {
        let amount = serde_cbor::from_slice::<u64>(state.get(&key.clone()).unwrap()).unwrap();
        new_total_supply += amount;
    }
    *state
        .entry([TOTAL_SUPPLY_KEY.to_vec(), token.to_vec()].concat().clone())
        .or_insert(Default::default()) = serde_cbor::to_vec(&new_total_supply).unwrap();
}

fn remove_stolen_funds(
    state: &mut HashMap<Vec<u8>, Vec<u8>>,
    token: [u8; 20],
    mut total_supply_after_hack: u64,
) {
    let total_supply = serde_cbor::from_slice::<u64>(
        state
            .get(&[TOTAL_SUPPLY_KEY.to_vec(), token.to_vec()].concat())
            .unwrap(),
    )
    .unwrap();
    let eth_balance_keys = state
        .keys()
        .cloned()
        .filter(|key| key.starts_with(&TOKEN_BALANCE_KEY.to_vec()) && key[24..] == token)
        .collect::<Vec<Vec<u8>>>();
    let mut total_hacker_balance = 0;
    for key in &eth_balance_keys {
        if V2_HACKER_ADDRESSES.contains(&key[4..24].try_into().unwrap()) {
            total_hacker_balance +=
                serde_cbor::from_slice::<u64>(state.get(&key[..]).unwrap()).unwrap();
        }
    }
    total_supply_after_hack += total_hacker_balance;
    let percentage = total_supply_after_hack * BASE_FACTOR / total_supply;
    let mut new_total_supply = 0;

    for key in &eth_balance_keys {
        let amount = serde_cbor::from_slice::<u64>(state.get(&key.clone()).unwrap()).unwrap();
        new_total_supply += amount * percentage / BASE_FACTOR;
        *state.get_mut(&key.clone()).unwrap() =
            serde_cbor::to_vec(&(amount * percentage / BASE_FACTOR)).unwrap();
    }
    *state
        .get_mut(&[TOTAL_SUPPLY_KEY.to_vec(), token.to_vec()].concat())
        .unwrap() = serde_cbor::to_vec(&new_total_supply).unwrap();
    let pool_supply_of_token = serde_cbor::from_slice::<u64>(
        state
            .get(&[POOL_SUPPLY_OF_TOKEN_KEY.to_vec(), token.to_vec()].concat())
            .unwrap(),
    )
    .unwrap();
    let pool_supply_of_base_token = serde_cbor::from_slice::<u64>(
        state
            .get(&[POOL_SUPPLY_OF_BASE_TOKEN_KEY.to_vec(), token.to_vec()].concat())
            .unwrap(),
    )
    .unwrap();
    let new_pool_supply_of_token = pool_supply_of_token * percentage / BASE_FACTOR;
    let new_pool_supply_of_base_token = pool_supply_of_base_token * percentage / BASE_FACTOR;

    *state
        .get_mut(&[POOL_SUPPLY_OF_TOKEN_KEY.to_vec(), token.to_vec()].concat())
        .unwrap() = serde_cbor::to_vec(&new_pool_supply_of_token).unwrap();
    *state
        .get_mut(&[POOL_SUPPLY_OF_BASE_TOKEN_KEY.to_vec(), token.to_vec()].concat())
        .unwrap() = serde_cbor::to_vec(&new_pool_supply_of_base_token).unwrap();
}

fn strip_unknown_balances(state: &mut HashMap<Vec<u8>, Vec<u8>>) {
    let balance_keys = state
        .keys()
        .cloned()
        .filter(|key| key.starts_with(&TOKEN_BALANCE_KEY.to_vec()))
        .collect::<Vec<Vec<u8>>>();
    for key in balance_keys {
        if ![USD, BTC, ETH, MS].contains(&key[24..].try_into().unwrap()) {
            state.remove(&key);
        }
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
        if transaction.id >= 3104941 {
            break;
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

fn is_usd(mut key: Vec<u8>) -> bool {
    if key.starts_with(b"Bridge") {
        key.drain(..6);
        let (token, _address) = key.split_at(20);
        return token == V1_USD;
    }
    false
}

fn is_liquidity_token(key: &[u8]) -> bool {
    key.starts_with(b"Exchange")
}

fn scale_usd_amount(value: &[u8]) -> Vec<u8> {
    let amount: u64 = serde_cbor::from_slice(value).unwrap();
    let scaled_amount = (BigInt::from(amount) * BigInt::from(10u128.pow(28)) / USD_EXCHANGE_RATE)
        .to_u64()
        .unwrap();
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
        } else if sha256(b"EllipticoinELC".to_vec()).to_vec() == key[8..].to_vec() {
            V2_ELC.to_vec()
        } else {
            key[8..].to_vec()
        }
    } else if key.starts_with(b"Bridge") {
        if key[6..].to_vec() == V1_ETH {
            V2_ETH.to_vec()
        } else if key[6..].to_vec() == V1_BTC {
            V2_BTC.to_vec()
        } else if key[6..].to_vec() == V1_USD {
            V2_USD.to_vec()
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
    serde_cbor::to_vec(
        &liquidity_providers
            .iter()
            .map(|address| {
                if let PublicKey(public_key) = address {
                    public_key[..20].try_into().unwrap()
                } else {
                    panic!("")
                }
            })
            .collect::<Vec<[u8; 20]>>(),
    )
    .unwrap()
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
