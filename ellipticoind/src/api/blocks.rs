use crate::api::helpers::{bytes_from_signed_data, bytes_from_signed_iterable};
use crate::api::types::BlockResult::{NotConsidered, Rejected, Witnessed};
use crate::api::types::{BlockResult, Bytes};
use crate::consensus::MinerBlockDecision::{Accepted, Burned};
use crate::consensus::{ExpectedBlock, MinerBlockDecision};
use crate::constants::{BLOCK_BROADCASTER, BLOCK_CHANNEL, NEXT_BLOCK};
use crate::models;
use crate::system_contracts::ellipticoin::Miner;
use ellipticoin::{BurnProofs, PublicKey, WitnessedMinerBlock};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<i64>,
}

pub async fn broadcaster(_req: tide::Request<()>, sender: tide::sse::Sender) -> tide::Result<()> {
    let mut block_broadcaster = BLOCK_BROADCASTER.clone();
    while let Some(event) = block_broadcaster.recv().await {
        sender
            .send("block", event.to_string(), Some(&event.to_string()))
            .await?;
    }
    Ok(())
}

pub async fn process_received_block(
    received_block: Bytes,
    block: models::block::Block,
    txs: Vec<models::transaction::Transaction>,
    signer_address: PublicKey,
) -> BlockResult {
    let mut next_block: ExpectedBlock;
    loop {
        let next_block_read: ExpectedBlock = (*NEXT_BLOCK.read().await).clone().unwrap();
        let next_block_miner: Miner = next_block_read.miner.clone();
        let next_block_number: i32 = next_block_read.number.clone();

        match get_decided_block_result(block.number, &signer_address, next_block_read).await {
            Some(NotConsidered()) => return NotConsidered(),
            Some(Witnessed(x)) => return Witnessed(x),
            Some(Rejected(x)) => return Rejected(x),
            _ => (),
        }

        next_block = (*NEXT_BLOCK.write().await).clone().unwrap();
        if next_block.number == next_block_number && next_block.miner == next_block_miner {
            break;
        }
    }

    match next_block.decisions.get(&next_block.miner.address) {
        Some(x) => {
            return match x {
                Burned(proof) => Rejected(bytes_from_signed_iterable(proof.values())),
                Accepted(witnessed_block) => Witnessed(bytes_from_signed_data(witnessed_block)),
            }
        }
        None => (),
    }

    if next_block.is_miner_burned_by_me(&next_block.miner).await {
        return Rejected(bytes_from_signed_iterable(
            next_block.current_burn_proof().unwrap().values(),
        ));
    }

    let block_u8: &[u8] = &(Vec::from(received_block));
    let witnessed_block: WitnessedMinerBlock = next_block.witness_miner_block(block_u8);

    // TODO: Save witness here

    let miner = block.apply(txs).await;
    next_block.increment(miner);
    BLOCK_CHANNEL.0.send(next_block.number).await;

    Witnessed(bytes_from_signed_data(&witnessed_block))
}

pub async fn get_decided_block_result(
    block_number: i32,
    block_miner_address: &PublicKey,
    next_block: ExpectedBlock,
) -> Option<BlockResult> {
    let next_block_miner: Miner = next_block.miner.clone();
    let next_block_number: i32 = next_block.number.clone();
    let burned_by_me: bool = next_block.is_miner_burned_by_me(&next_block_miner).await;
    let burn_proof: Option<BurnProofs> = next_block.current_burn_proof();
    let current_miner_decision: Option<&MinerBlockDecision> =
        next_block.decisions.get(&next_block_miner.address);
    let req_miner_decision: Option<&MinerBlockDecision> =
        next_block.decisions.get(block_miner_address);

    if next_block_miner.address != block_miner_address.clone() || next_block_number != block_number
    {
        if block_number < next_block_number {
            // TODO: Send witness / rejection from storage
            Some(Witnessed(Bytes("derp".as_bytes().to_vec())))
        } else if block_number > next_block_number {
            Some(NotConsidered())
        } else {
            Some(get_res_from_decision(req_miner_decision.clone()))
        }
    } else {
        match get_res_from_decision(current_miner_decision) {
            NotConsidered() => {
                if burned_by_me {
                    let bytes = bytes_from_signed_iterable(burn_proof.unwrap().values());
                    Some(Rejected(bytes))
                } else {
                    Some(NotConsidered())
                }
            }
            x => Some(x),
        }
    }
}

fn get_res_from_decision(decision: Option<&MinerBlockDecision>) -> BlockResult {
    return match decision {
        Some(Burned(burn_txs)) => Rejected(bytes_from_signed_iterable(burn_txs.values())),
        Some(Accepted(witnessed_block)) => Witnessed(bytes_from_signed_data(witnessed_block)),
        None => NotConsidered(),
    };
}
