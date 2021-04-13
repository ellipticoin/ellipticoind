pub mod constants;
pub mod transaction;
use ellipticoin_types::Address;
pub use transaction::*;

use crate::constants::{
    BASE_TOKEN_ADDRESS, BRIDGE_ADDRESS, DECIMALS, ELLIPTICOIN_DECIMALS, ETH_ADDRESS,
    EXCHANGE_RATE_CURRENT_SELECTOR, RECEIVED_ETH_TOPIC, REDEEM_TOPIC,
    SUPPLY_RATE_PER_BLOCK_SELECTOR, TOKENS, TRANSFER_TOPIC, WEB3_URL,
};
use ellipticoin_contracts::{
    bridge::{Mint, Redeem, Update},
    constants::BASE_FACTOR,
};
use num_bigint::BigInt;
use num_traits::{pow::pow, ToPrimitive};
use serde_json::{json, Value};
use std::{collections::HashMap, convert::TryInto, task::Poll};
use surf;
pub use transaction::ecrecover;

pub async fn poll(latest_block: u64) -> Result<Poll<Update>, surf::Error> {
    let current_block = get_current_block().await?;
    if current_block == latest_block {
        Ok(Poll::Pending)
    } else {
        let base_token_exchange_rate = eth_call(
            BASE_TOKEN_ADDRESS,
            EXCHANGE_RATE_CURRENT_SELECTOR,
            current_block,
        )
        .await?;
        let base_token_interest_rate = get_base_token_interest_rate(current_block).await.unwrap();

        // Ethereum nodes only store 128 blocks of history.
        // If we're greater than 128 blocks behind assume there was a restart
        // and skip to the current block.
        let from_block = if current_block - latest_block > 128 {
            current_block
        } else {
            latest_block + 1
        };
        Ok(Poll::Ready(Update {
            block_number: current_block,
            base_token_interest_rate,
            base_token_exchange_rate,
            mints: [
                get_eth_mints(from_block, current_block).await?,
                get_token_mints(from_block, current_block).await?,
            ]
            .concat(),
            redeems: get_redeems(from_block, current_block).await?,
        }))
    }
}

async fn get_token_mints(from_block: u64, to_block: u64) -> Result<Vec<Mint>, surf::Error> {
    let logs = get_logs(from_block, to_block, vec![TRANSFER_TOPIC]).await?;
    Ok(logs
        .iter()
        .filter_map(|log| {
            let topics = log.get("topics").unwrap();
            let token_address =
                parse_address(&value_to_string(&log.get("address").unwrap()).unwrap()).unwrap();
            let to = match value_to_string(&topics[2])
                .as_ref()
                .and_then(|address| parse_address(address))
            {
                Some(to) => to,
                None => return None,
            };
            if TOKENS.contains(&token_address) && to == BRIDGE_ADDRESS {
                let from = parse_address(&value_to_string(&topics[1]).unwrap()).unwrap();
                let amount = match parse_big_int(
                    &value_to_string(&log.get("data").unwrap().clone()).unwrap(),
                ) {
                    Some(amount) => amount,
                    None => return None,
                };
                Some(Mint(
                    scale_down(amount, *DECIMALS.get(&token_address).unwrap()),
                    token_address,
                    from,
                ))
            } else {
                None
            }
        })
        .collect())
}

async fn get_eth_mints(from_block: u64, to_block: u64) -> Result<Vec<Mint>, surf::Error> {
    let logs = get_logs(from_block, to_block, vec![RECEIVED_ETH_TOPIC]).await?;
    Ok(logs
        .iter()
        .filter_map(|log| {
            let topics = log.get("topics").unwrap();
            let address =
                parse_address(&value_to_string(&log.get("address").unwrap()).unwrap()).unwrap();
            if address == BRIDGE_ADDRESS {
                let from = parse_address(&value_to_string(&topics[1]).unwrap()).unwrap();
                let amount = match parse_big_int(
                    &value_to_string(&log.get("data").unwrap().clone()).unwrap(),
                ) {
                    Some(amount) => amount,
                    None => return None,
                };
                Some(Mint(
                    scale_down(amount, *DECIMALS.get(&ETH_ADDRESS).unwrap()),
                    ETH_ADDRESS,
                    from,
                ))
            } else {
                None
            }
        })
        .collect())
}

async fn get_redeems(from_block: u64, to_block: u64) -> Result<Vec<Redeem>, surf::Error> {
    let logs = get_logs(from_block, to_block, vec![REDEEM_TOPIC]).await?;
    Ok(logs
        .iter()
        .filter_map(|log| {
            let address =
                parse_address(&value_to_string(&log.get("address").unwrap()).unwrap()).unwrap();
            if address == BRIDGE_ADDRESS {
                let redeem_id =
                    parse_big_int(&value_to_string(&log.get("data").unwrap().clone()).unwrap())
                        .unwrap()
                        .to_u64()
                        .unwrap();
                Some(Redeem(redeem_id))
            } else {
                None
            }
        })
        .collect())
}

fn scale_down(amount: BigInt, decimals: usize) -> u64 {
    (amount / BigInt::from(pow(BigInt::from(10), decimals - *ELLIPTICOIN_DECIMALS)))
        .to_u64()
        .unwrap()
}

pub async fn get_base_token_interest_rate(block_number: u64) -> Result<u64, surf::Error> {
    let rate = eth_call(
        BASE_TOKEN_ADDRESS,
        SUPPLY_RATE_PER_BLOCK_SELECTOR,
        block_number,
    )
    .await
    .unwrap();
    let mantissa = pow(10f64, 18);
    let blocks_per_day = 4 * 60 * 24;
    let days_per_year = 365;
    let apy_as_percentage = ((pow(
        (rate.to_u64().unwrap() as f64 / mantissa as f64 * blocks_per_day as f64) + 1f64,
        days_per_year,
    )) - 1f64)
        * 100f64;
    Ok((apy_as_percentage * (BASE_FACTOR as f64)) as u64)
}

pub async fn eth_call(
    contract_address: Address,
    selector: [u8; 4],
    block_number: u64,
) -> Result<BigInt, surf::Error> {
    let res_hex = loop {
        let mut res = match surf::post(WEB3_URL.clone())
            .body(json!(
             {
             "id": 1,
             "jsonrpc": "2.0",
             "method": "eth_call",
             "params": [
                 {
                     "to": format!("0x{}", hex::encode(contract_address)),
                     "data": format!("0x{}", hex::encode(selector)),
                 },
                 format!("0x{}", BigInt::from(block_number).to_str_radix(16))
             ]}
            ))
            .await
        {
            Ok(res_hex) => res_hex,
            Err(_) => continue,
        };
        let res_hash_map = match res.body_json::<HashMap<String, serde_json::Value>>().await {
            Ok(res_hash_map) => res_hash_map,
            Err(_) => continue,
        };
        if res_hash_map.contains_key("result") {
            break serde_json::from_value::<String>(res_hash_map.get("result").unwrap().clone())?;
        }
    };

    Ok(BigInt::parse_bytes(res_hex.trim_start_matches("0x").as_bytes(), 16).unwrap())
}

pub async fn get_current_block() -> Result<u64, surf::Error> {
    let res_hex = loop {
        let mut res = match surf::post(WEB3_URL.clone())
            .body(json!(
                {
                    "id": 1,
                    "jsonrpc": "2.0",
                    "method": "eth_blockNumber",
                    "params": []
                }
            ))
            .await
        {
            Ok(res) => res,
            Err(_) => continue,
        };
        let res_hex = match res.body_json::<HashMap<String, serde_json::Value>>().await {
            Ok(res_hashmap) => {
                serde_json::from_value::<String>(res_hashmap.get("result").unwrap().clone())
                    .expect("error converting to hash")
            }
            Err(_) => continue,
        };
        if !(res_hex == "0x0") {
            break res_hex;
        }
    };

    Ok(
        BigInt::parse_bytes(res_hex.trim_start_matches("0x").as_bytes(), 16)
            .unwrap()
            .to_u64()
            .unwrap(),
    )
}

async fn get_logs(
    from_block: u64,
    to_block: u64,
    topics: Vec<[u8; 32]>,
) -> Result<Vec<Value>, surf::Error> {
    loop {
        let mut res = match surf::post(WEB3_URL.clone())
        .body(json!(
            {
                "id": 1,
                "jsonrpc": "2.0",
                "method": "eth_getLogs",
                "params": [{
                    "fromBlock": format!("0x{}", BigInt::from(from_block).to_str_radix(16)),
                    "toBlock": format!("0x{}", BigInt::from(to_block).to_str_radix(16)),
                    "topics": topics.iter().map(|topic| format!("0x{}", hex::encode(topic))).collect::<Vec<String>>(),
                }]
            }
        ))
        .await {
            Ok(res) => res,
            Err(err) =>  {
                println!("{}", err);
                continue
            }
        };
        let body_json = res
            .body_json::<HashMap<String, serde_json::Value>>()
            .await?;
        let result = match body_json.get("result") {
            Some(res) => res.clone(),
            None => {
                panic!("{:?}", body_json)
            }
        };
        match serde_json::from_value(result) {
            Ok(res) => break Ok(res),
            Err(err) => {
                println!("{}", err);
                continue;
            }
        }
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    serde_json::from_value(value.clone()).ok()
}

fn parse_address(s: &str) -> Option<Address> {
    if s == "0x" {
        return None;
    }

    let bytes = hex::decode(s.trim_start_matches("0x")).unwrap();
    Some(Address(bytes[..][bytes.len() - 20..].try_into().unwrap()))
}
fn parse_big_int(s: &str) -> Option<BigInt> {
    if s == "0x" {
        return None;
    }

    Some(BigInt::parse_bytes(s.trim_start_matches("0x").as_bytes(), 16).unwrap())
}
