pub mod constants;
pub mod transaction;
pub use transaction::*;

use constants::{
    BRIDGE_ADDRESS, DECIMALS, ELLIPTICOIN_DECIMALS, ETH_ADDRESS, RECEIVED_ETH_TOPIC, REDEEM_TOPIC,
    TOKENS, TRANSFER_TOPIC, WEB3_URL,
};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde_json::{json, Value};
use std::{collections::HashMap, convert::TryInto, task::Poll};
use surf;

#[derive(Clone, Debug)]
pub struct Mint(pub u64, pub [u8; 20], pub [u8; 20]);
#[derive(Clone, Debug)]
pub struct Redeem(pub u64);

pub struct Update {
    pub block_number: u64,
    pub mints: Vec<Mint>,
    pub redeems: Vec<Redeem>,
}
pub async fn poll(latest_block: u64) -> Result<Poll<Update>, surf::Error> {
    println!("before current block");
    let current_block = get_current_block().await?;
    println!("after current block");
    if current_block == latest_block {
        return Ok(Poll::Pending);
    }
    Ok(Poll::Ready(Update {
        block_number: current_block,
        mints: [
            get_eth_mints(latest_block + 1, current_block).await?,
            get_token_mints(latest_block + 1, current_block).await?,
        ]
        .concat(),
        redeems: get_redeems(latest_block + 1, current_block).await?,
    }))
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
                Some(Mint(scale_down(amount, token_address), token_address, from))
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
                Some(Mint(scale_down(amount, ETH_ADDRESS), ETH_ADDRESS, from))
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

fn scale_down(amount: BigInt, token: [u8; 20]) -> u64 {
    (amount / 10usize.pow(*DECIMALS.get(&token).unwrap() as u32 - *ELLIPTICOIN_DECIMALS as u32))
        .to_u64()
        .unwrap()
}

pub async fn get_current_block() -> Result<u64, surf::Error> {
    let mut res_hex;
    loop {
        let mut res = surf::post(WEB3_URL.clone())
            .body(json!(
                {
                    "id": 1,
                    "jsonrpc": "2.0",
                    "method": "eth_blockNumber",
                    "params": []
                }
            ))
            .await?;
        res_hex = serde_json::from_value::<String>(
            res.body_json::<HashMap<String, serde_json::Value>>()
                .await?
                .get("result")
                .unwrap()
                .clone(),
        )
        .unwrap();
        if !(res_hex == "0x0") {
            break;
        }
    }
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
    let mut res = surf::post(WEB3_URL.clone())
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
        .await?;
    let body_json = res
        .body_json::<HashMap<String, serde_json::Value>>()
        .await?;
    Ok(serde_json::from_value(
        body_json
            .get("result")
            .expect(&format!("{:?}", body_json))
            .clone(),
    )
    .unwrap())
}

fn value_to_string(value: &Value) -> Option<String> {
    serde_json::from_value(value.clone()).ok()
}

fn parse_address(s: &str) -> Option<[u8; 20]> {
    if s == "0x" {
        return None;
    }

    let bytes = hex::decode(s.trim_start_matches("0x")).unwrap();
    Some(bytes[..][bytes.len() - 20..].try_into().unwrap())
}
fn parse_big_int(s: &str) -> Option<BigInt> {
    if s == "0x" {
        return None;
    }

    Some(BigInt::parse_bytes(s.trim_start_matches("0x").as_bytes(), 16).unwrap())
}
