use crate::{
    api::{
        graphql::{Context, Error},
        helpers::validate_signature,
        types::{Bytes, Transaction, PostBlockResult},
    },
    constants::{BLOCK_CHANNEL, MINERS, NEXT_BLOCK},
    helpers::run_transaction,
    models,
    system_contracts::ellipticoin::Miner,
};
use crate::api::types::PostBlockResult::{Witnessed, NotConsidered, Rejected};
use crate::consensus::{MinerBlockDecision, ExpectedBlock};
use ellipticoin::{PublicKey, BurnProofs, WitnessedMinerBlock};
use crate::consensus::MinerBlockDecision::{Accepted, Burned};
use crate::api::types::GraphQLPostBlockResult;
use serde_cose::Sign1;

pub struct Mutations;

#[juniper::graphql_object(
    Context = Context,
)]
impl Mutations {
    pub async fn post_transaction(
        _context: &Context,
        transaction: Bytes,
    ) -> Result<Transaction, Error> {
        let (transaction_request, _) = validate_signature(&transaction.0)?;
        let transaction = run_transaction(transaction_request).await;
        Ok(Transaction::from(transaction))
    }

    pub async fn post_block(_context: &Context, posted_block: Bytes) -> Result<GraphQLPostBlockResult, Error> {
        let ((block, txs), signer_address): ((models::block::Block, Vec<models::transaction::Transaction>), PublicKey) =
            validate_signature(&posted_block.0)?;

        let res: PostBlockResult = get_post_block_result(posted_block, block, txs, signer_address).await;
        Ok(GraphQLPostBlockResult::from(res))
    }

    pub async fn slash_winner(_context: &Context, block: Bytes) -> Result<bool, Error> {
        let ((message, winner), _): ((String, PublicKey), _) = validate_signature(&block.0)?;
        if message != "Slash" {
            return Err(Error("Message didn't start with \"Slash\"".to_string()));
        }
        if MINERS.current().await.address == winner {
            println!("Slash winner")
        }
        Ok(true)
    }
}

async fn get_post_block_result(posted_block: Bytes, block: models::block::Block, txs: Vec<models::transaction::Transaction>, signer_address: PublicKey) -> PostBlockResult {
    let mut next_block: ExpectedBlock;
    loop {
        let next_block_read: ExpectedBlock = (*NEXT_BLOCK.read().await).clone().unwrap();
        let next_block_miner: Miner = next_block_read.miner.clone();
        let next_block_number: i32 = next_block_read.number.clone();

        match get_decided_block_result(&block, &signer_address, next_block_read).await {
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
        Some(x) => return match x {
            Burned(proof) => Rejected(bytes_from_signed_txs(proof.values())),
            Accepted(witnessed_block) => Witnessed(bytes_from_signed_tx(witnessed_block)),
        },
        None => ()
    }

    if next_block.is_miner_burned_by_me(&next_block.miner).await {
        return Rejected(bytes_from_signed_txs(next_block.current_burn_proof().unwrap().values()))
    }

    let block_u8: &[u8] = &(Vec::from(posted_block));
    let witnessed_block: WitnessedMinerBlock = next_block.witness_miner_block(block_u8);

    // TODO: Save witness here

    let miner = block.apply(txs).await;
    BLOCK_CHANNEL.0.send(miner.clone()).await;

    next_block.increment(miner);

    Witnessed(bytes_from_signed_tx(&witnessed_block))
}

async fn get_decided_block_result(block: &models::block::Block, req_miner_address: &PublicKey, next_block: ExpectedBlock) -> Option<PostBlockResult> {
    let next_block_miner: Miner = next_block.miner.clone();
    let next_block_number: i32 = next_block.number.clone();
    let burned_by_me: bool = next_block.is_miner_burned_by_me(&next_block_miner).await;
    let burn_proof: Option<BurnProofs> = next_block.current_burn_proof();
    let current_miner_decision: Option<&MinerBlockDecision> = next_block.decisions.get(&next_block_miner.address);
    let req_miner_decision: Option<&MinerBlockDecision> = next_block.decisions.get(req_miner_address);


    return if next_block_miner.address != req_miner_address.clone() || next_block_number != block.number {
        if block.number < next_block_number {
            // TODO: Send witness / rejection
            Some(Witnessed(Bytes("derp".as_bytes().to_vec())))
        } else if block.number > next_block_number {
            Some(NotConsidered())
        } else {
            Some(get_res_from_decision(req_miner_decision.clone()))
        }
    } else {
        match get_res_from_decision(current_miner_decision) {
            NotConsidered() => match burned_by_me {
                true => {
                    let bytes = bytes_from_signed_txs(burn_proof.unwrap().values());
                    Some(Rejected(bytes))
                },
                false => Some(NotConsidered())
            }
            x => Some(x)
        }
    }
}

fn get_res_from_decision(decision: Option<&MinerBlockDecision>) -> PostBlockResult {
    return match decision {
        Some(Burned(burn_txs)) => Rejected(bytes_from_signed_txs(burn_txs.values())),
        Some(Accepted(witnessed_block)) => Witnessed(bytes_from_signed_tx(witnessed_block)),
        None => NotConsidered()
    }
}

fn bytes_from_signed_txs<'a, I>(signed_txs: I) -> Vec<Bytes>
    where
        I: Iterator<Item = &'a Sign1>,
{
    signed_txs
        .map(|t| bytes_from_signed_tx(t))
        .collect()
}

fn bytes_from_signed_tx(signed_tx: &Sign1) -> Bytes {
    Bytes::from(serde_cbor::to_vec(signed_tx).unwrap())
}
