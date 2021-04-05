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
use std::time::SystemTime;

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

const HACKER_ADDRESSES: [[u8; 32]; 24] = [
    hex!("39a2a6c025ab5217420ac20c37f56255cbb5b8117454cdc55b7322d73fd54cdb"),
    hex!("bd0327dc9bd2dc04c84ed790fa03987ee5529f66eeb801fee1ef0d63f0afb700"),

    hex!("b3fa7979614109d20b32da16854c57f803d62a4c66809790f25913714a831615"),
    hex!("1fb0c9ea9d1f0aa2a82afb7ccdebf0061b1aa0e05480538a777efbee77900a28"),
    hex!("4288e32b510f7f204be44d4671d3284582874d5dc3bfb5a8a74154ea639f58f5"),
    hex!("e3886b6c604a20c21a3a24b509ac658f3763d04e53276bf8580ef39a426b5fdf"),
    hex!("86b2d5e924c3a7c3e32929833219ec7eb04e93c02e9c41ccd75f51744ba0f881"),
    hex!("1a6e325901190934dab08e306938d4221b12050df83fb966b8b9f4f8877f37be"),
    hex!("7c7ca82a864a71810c27d1dc1df20af94ecef7b8d3a26b1172537d5be7584670"),
    // hex!("e9c55c31201f51ec7cc59e0f4b5b1ec1317edf91ed73d1873a6e54e036d58bfe"),
    // hex!("37471d396a424267b2e6b80f9eadbf8b08c84b756e4253e000222f26b0db21bc"),
    // hex!("df0d4428443fdcacca679e754526394c3d78ceb82046b2c514fbdc26e89c3bf5"),
    // hex!("6f0fc7010b611f9011ff354c1c8ddbda371eab1ee8ed05f85f5f449d021c51aa"),
    // hex!("e9e096ec1d5d1545b0ebcb26f5ff61669eaeac6d8209f113f4a737a4fd2956a8"),
    // hex!("f3ea1ab0b3bc5d1c03c42f73419a53dfa6c44efff6bd596423fba48e341bb191"),
    hex!("9838d62618f0e413fbdc199a1d0667bf9a6c0a2e17cefed6bb95f27b39643500"),
    hex!("7383559bc1749f410af5212a1b15c013d95581432f6e50574e629d7cec3dbd80"),
    hex!("1a8e4e6c86411acbeab952539139208cf9dfd14c3bcb2d70abeb2771fc94a95a"),
    // hex!("11abc7b344f9f22ec3b0c6c6ceaf7ed63de56f50f688f57e558fd0d0294c1079"),
    // hex!("9b56e69b4d46ab0790c16cde91a04dbef8cd14e6af442c33b2f3275ce29873b7"),
    hex!("31547fbe32bf56fce8b8a36a041e0ee77f9c0b38241bc770c9de6a72dadf6c0b"),
    hex!("31547fbe32bf56fce8b8a36a041e0ee77f9c0b38241bc770c9de6a72dadf6c0b"),
    // hex!("a452faaae70a5e4f83bcbf8a0d6f02300480ffabf59f4860292cbf8c5ff68331"),
    // hex!("2c6b2f424e6d15548332fa5ded59d0851a8b59488296b341bee5b41bdc52bae2"),
    hex!("08bf9c2de3a75b524f12f05341be7b20ba03dc8bcbf69e58c5d7d4508f8caf76"),
    hex!("404ebbaec191f0d57c2226b93d4931e9d6635dd823e5abfd6324d99cfee69fba"),
    hex!("5906962a12395c34967527be23621d835e48c8c650f1ca4814acb30341d24044"),
    hex!("29d65760101bb1b8cd300cafef7346de80a4ea34fe1741c205f790697aa05506"),
    // hex!("f55b8dcbf58f2dcb5279c8738f26043c53cede95906376649d7dc50c83d7cdec"),
    // hex!("b9a3ff3f0ac9a83f142b43115ad148145b7821d64a35b09a34e525584dc2bc88"),
    hex!("1dbb6765e6335abc512423ff60913e6b25b133a8b80d070e628db3f161ee6b6b"),
    hex!("0824073c36fbeaa0f32e2ecdd94c789b55ac8e17cb0b5606e031314bcf0a4500"),
    hex!("df0d4428443fdcacca679e754526394c3d78ceb82046b2c514fbdc26e89c3bf5"),
    hex!("4407adb31ed24d49fea74ce1fccfc2718c6f402da7d149b12e85da09c9d58aa9"),
    hex!("d8b5d65af4c87a05f0d713090da2718323460382620390f04ad7854f677ebbae"),
    hex!("44cb7c96b48670dab5d7c1e7f156ab448ff95dd68ade6e779fb493af4f66079f"),
];

lazy_static! {
    static ref V2_HACKER_ADDRESSES: Vec<[u8; 20]> = HACKER_ADDRESSES.iter().map(|address| <[u8; 20]>::try_from(address[..20].to_vec()).unwrap()).collect::<Vec<[u8; 20]>>();
}

const V1_BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const V1_ETH: [u8; 20] = hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
const V2_BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const V2_ETH: [u8; 20] = hex!("0000000000000000000000000000000000000000");
const V2_ELC: [u8; 20] = hex!("0000000000000000000000000000000000000002");
const V1_USD: [u8; 20] = hex!("6b175474e89094c44da98b954eedeac495271d0f");
const V2_USD: [u8; 20] = hex!("5d3a536e4d6dbd6114cc1ead35777bab948e3643");
// const V2_USD: [u8; 20] = hex!("6d7f0754ffeb405d23c51ce938289d4835be3b14");
const USD_EXCHANGE_RATE: u128 = 211367456115200165329965416;
const TOKEN_BALANCE_KEY: [u8; 4] = [6, 0, 0, 0];
const LIQUIDITY_BALANCE_KEY: [u8; 4] = [0, 0, 0, 0];
const POOL_SUPPLY_OF_BASE_TOKEN_KEY: [u8; 4] = [0, 0, 2, 0];
const POOL_SUPPLY_OF_TOKEN_KEY: [u8; 4] = [0, 0, 3, 0];
const BALANCE_OF_LIQUIDITY_TOKEN: [u8; 4] = [0, 0, 0, 0];
const TOTAL_SUPPLY_KEY: [u8; 4] = [6, 0, 3, 0];
const AMM_ADDRESS: [u8; 20] = [0; 20];
const BASE_FACTOR: u64 = 1000000;
const BTC: [u8; 20] = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
const ETH: [u8; 20] = hex!("0000000000000000000000000000000000000000");
const WETH: [u8; 20] = hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
const MS: [u8; 20] = hex!("0000000000000000000000000000000000000002");
const USD: [u8; 20] = hex!("5d3a536e4d6dbd6114cc1ead35777bab948e3643");
// It looks as though ETH has been being stolen since the network launched
// This is the amount that was left after the hack
// https://etherscan.io/tx/0xbb3e03c5cb6804bf5d19167123285c23518dd06a47ac3de84e46e69296045265
const ETH_TOTAL_SUPPLY_AFTER_HACK: u64 = 4263245;


// 14K DAI was stolen out of the bridge using feeless 1 unit transactions
// https://etherscan.io/tx/0x91baf78c28ff576607a6723e10b164161e9f72ad38863460150d9d275ea25ecb
// 0.4175095-(14504.171817/59291.90/2) = 0.29519792104
const BTC_TOTAL_SUPPLY_AFTER_HACK: u64 = 295197;

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
    let mut state = IN_MEMORY_STATE.lock().await;
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
    remove_stolen_funds(&mut v2_genesis_state, ETH, ETH_TOTAL_SUPPLY_AFTER_HACK);
    remove_stolen_funds(&mut v2_genesis_state, BTC, BTC_TOTAL_SUPPLY_AFTER_HACK);
    strip_unknown_balances(&mut state);
    let dao_address: [u8; 20] = pad_left(vec![V2Contracts::Governance as u8], 20)
                .try_into()
                .unwrap();
    let now: u64 = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let additional_seed_amount = ((now - TIME_OF_HACK)/4) * 1280000;
    let dao_seed_amount = BASE_DAO_SEED_AMOUNT + additional_seed_amount;
    credit(&mut state, dao_address, dao_seed_amount, MS);

    for (key, value) in v2_genesis_state.iter() {
        serde_cbor::to_writer(&file, &(key, value)).unwrap();
    }
}

fn credit(state: &mut  HashMap<Vec<u8>, Vec<u8>>, address: [u8; 20], amount: u64, token: [u8; 20]) {
    let balance_key =  [TOKEN_BALANCE_KEY.to_vec(), address.to_vec(), token.to_vec()].concat();
    let balance = serde_cbor::from_slice::<u64>(state.get(&balance_key).unwrap_or(&vec![])).unwrap_or(0);
    *state.entry(balance_key.clone()).or_insert(Default::default()) = serde_cbor::to_vec(&(balance + amount)).expect("boom");
}

fn remove_stolen_funds(state: &mut HashMap<Vec<u8>, Vec<u8>>, token: [u8; 20], mut total_supply_after_hack: u64) {
    let total_supply = serde_cbor::from_slice::<u64>(state.get(&[TOTAL_SUPPLY_KEY.to_vec(), token.to_vec()].concat()).unwrap()).unwrap();
    let eth_balance_keys = state.keys().cloned().filter(|key| key.starts_with(&TOKEN_BALANCE_KEY.to_vec()) && key[24..] == token).collect::<Vec<Vec<u8>>>();
    let mut total_hacker_balance = 0;
    for key in &eth_balance_keys {
        if V2_HACKER_ADDRESSES.contains(&key[4..24].try_into().unwrap()) {
            total_hacker_balance += serde_cbor::from_slice::<u64>(state.get(&key[..]).unwrap()).unwrap(); 
        }
}
    total_supply_after_hack += total_hacker_balance;
    let percentage = total_supply_after_hack * BASE_FACTOR / total_supply;
    let mut new_total_supply = 0;
    
    for key in &eth_balance_keys {
       let amount = serde_cbor::from_slice::<u64>(state.get(&key.clone()).unwrap()).unwrap();
        new_total_supply += amount * percentage / BASE_FACTOR;
       *state.get_mut(&key.clone()).unwrap() = serde_cbor::to_vec(&(amount * percentage / BASE_FACTOR)).unwrap();
    }
    *state.get_mut(&[TOTAL_SUPPLY_KEY.to_vec(), token.to_vec()].concat()).unwrap() = serde_cbor::to_vec(&new_total_supply).unwrap();
    let pool_supply_of_token = serde_cbor::from_slice::<u64>(state.get(&[POOL_SUPPLY_OF_TOKEN_KEY.to_vec(), token.to_vec()].concat()).unwrap()).unwrap();
    let pool_supply_of_base_token = serde_cbor::from_slice::<u64>(state.get(&[POOL_SUPPLY_OF_BASE_TOKEN_KEY.to_vec(), token.to_vec()].concat()).unwrap()).unwrap();
    let new_pool_supply_of_token = pool_supply_of_token * percentage/ BASE_FACTOR; 
    let new_pool_supply_of_base_token = pool_supply_of_base_token * percentage/ BASE_FACTOR;

    *state.get_mut(&[POOL_SUPPLY_OF_TOKEN_KEY.to_vec(), token.to_vec()].concat()).unwrap() = serde_cbor::to_vec(&new_pool_supply_of_token).unwrap(); 
    *state.get_mut(&[POOL_SUPPLY_OF_BASE_TOKEN_KEY.to_vec(), token.to_vec()].concat()).unwrap() = serde_cbor::to_vec(&new_pool_supply_of_base_token).unwrap(); 
}

fn strip_unknown_balances(state: &mut  HashMap<Vec<u8>, Vec<u8>>) {
    let balance_keys = state.keys().cloned().filter(|key| key.starts_with(&TOKEN_BALANCE_KEY.to_vec())).collect::<Vec<Vec<u8>>>();
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
    // let mut expected_total_supply = 0;
    // let mut total_minted = 0;
    // let mut total_released = 0;
    // let magic_number = serde_cbor::value::to_value(51495).unwrap();
    for mut transaction in transactions {
        if transaction.id >= 3104941 {
            break;
        }
        let mut api = InMemoryAPI::new(&mut state, Some(transaction.clone().into()));

        legacy::run(&mut api, &mut transaction).await;
//         let balance = crate::system_contracts::token::get_balance(
//                 &mut api,
//                 ellipticoin::Token {
// issuer: "Bridge".into(),
// id: V1_ETH.to_vec().into(),
// },
//                 ellipticoin::Address::PublicKey(
//                     hex::decode("a79d7e37bf80d57e2f1d3c904b0d58d95c785af076a2a8e553f356977dc14f9e").unwrap().try_into().unwrap()),
// );
//             if balance > 0 {
//                 panic!("{}", transaction.id);
//             }
           //  let arguments: Vec<serde_cbor::Value> = serde_cbor::from_slice(&transaction.arguments).unwrap();
           // if arguments.len() == 3 {
           //  if arguments[2] == magic_number {
           //  println!("{}", transaction.id);
           //  }
           // }
//         if transaction.function == "release" {
            // println!("{} {} {} {:?}", transaction.function, total_supply_before, total_supply_after, return_value);
        // }
        
//         if ["mint", "release"].contains(&transaction.function.as_ref()) {
//             let token = serde_cbor::value::from_value::<serde_bytes::ByteBuf>(serde_cbor::from_slice::<Vec<serde_cbor::Value>>(&transaction.arguments).unwrap()[0].clone()).unwrap();
//             if token.to_vec() == V1_ETH {
//                 let amount = serde_cbor::value::from_value::<u64>(serde_cbor::from_slice::<Vec<serde_cbor::Value>>(&transaction.arguments).unwrap()[2].clone()).map(|amount| amount.to_string()).unwrap_or("NaN".to_string());
//                 let recipient = serde_cbor::value::from_value::<serde_bytes::ByteBuf>(serde_cbor::from_slice::<Vec<serde_cbor::Value>>(&transaction.arguments).unwrap()[1].clone()).unwrap();
//                 let return_value2: Result<serde_cbor::Value, serde_cbor::Value> = serde_cbor::value::from_value(return_value.clone()).unwrap();
//                 if return_value2.is_err() {
//                     println!("ERROR {} {} {}", transaction.function, amount, hex::encode(recipient.to_vec()));
//                 } else {
//                     if  transaction.function == "mint" {
//                         total_minted += amount.parse().unwrap_or(0);
//                     } else {
//                         total_minted -= amount.parse().unwrap_or(0);
//                     }
//         let total_supply_after = crate::system_contracts::token::get_total_supply(
//                 &mut api,
//                 ellipticoin::Token {
// issuer: "Bridge".into(),
// id: V1_ETH.to_vec().into(),
// },
// );
//                     println!("{} {} {} {} {}", transaction.function, amount, total_minted, total_supply_after, hex::encode(recipient.to_vec()));
//                 }
//             }
//         }
        //     // println!("is err {}", return_value2.is_err());
        //     if token.to_vec() == V1_ETH && !return_value2.is_err() {
        //         let amount = if transaction.function == "mint" {
        //            let amount = serde_cbor::value::from_value::<u64>(serde_cbor::from_slice::<Vec<serde_cbor::Value>>(&transaction.arguments).unwrap()[2].clone()).unwrap();
        //             expected_total_supply += amount;
        //             println!("mint {} {}", transaction.id, amount);
        //             amount
        //         } else {
        //           let amount = serde_cbor::value::from_value::<u64>(serde_cbor::from_slice::<Vec<serde_cbor::Value>>(&transaction.arguments).unwrap()[2].clone()).unwrap();
        //             expected_total_supply -= amount;
        //             println!("release {} {}", transaction.id, amount);
        //             amount
        //         };
        //         let total_supply = crate::system_contracts::token::get_total_supply(
        //                     &mut api,
        //                     ellipticoin::Token {
        //                         issuer: "Bridge".into(),
        //                         id: V1_ETH.to_vec().into(),
        //                     },
        //                 );
        //         // println!("id: {} ETH Total Supply {}", transaction.id, total_supply as f64 / BASE_FACTOR as f64);
        //         if expected_total_supply != total_supply {
        //             println!("{}", expected_total_supply);
        //             println!("{}", total_supply);
        //             panic!();
        //         }
        //     }
        // }
        // if transaction.function == "exchange" {
        //     let eth_price = crate::system_contracts::exchange::get_price(
        //                 &mut api,
        //                 ellipticoin::Token {
        //                     issuer: "Bridge".into(),
        //                     id: V1_ETH.to_vec().into(),
        //                 },
        //             )
        //             .unwrap_or(0);
        //     let btc_price = crate::system_contracts::exchange::get_price(
        //                 &mut api,
        //                 ellipticoin::Token {
        //                     issuer: "Bridge".into(),
        //                     id: V1_BTC.to_vec().into(),
        //                 },
        //             )
        //             .unwrap_or(0);
        //     println!("id: {} ETH Price {} BTC Price {}", transaction.id, eth_price as f64 / BASE_FACTOR as f64, btc_price as f64 / BASE_FACTOR as f64);
        // }
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
