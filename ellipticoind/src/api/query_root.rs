use crate::api::{
    graphql::Context,
    types::{self, *},
};
use crate::constants::DB;
use anyhow::anyhow;
use ellipticoin_contracts::{
    bridge,
    constants::{BASE_FACTOR, MS, USD},
    governance, Bridge, Ellipticoin, Governance, System, AMM,
};
use ellipticoin_peerchain_ethereum::constants::BRIDGE_ADDRESS;

use juniper::FieldError;
use std::convert::{TryFrom, TryInto};

pub struct QueryRoot;
#[juniper::graphql_object(
    Context = Context,
)]
impl QueryRoot {
    async fn bridge(_context: &Context) -> types::Bridge {
        types::Bridge {
            address: Address(BRIDGE_ADDRESS),
            signers: vec![], //.iter().map(|signer| Bytes(signer)).collect()
        }
    }

    async fn tokens(
        _context: &Context,
        tokens: Vec<Address>,
        address: Address,
    ) -> Result<Vec<Token>, FieldError> {
        let mut db = DB.get().unwrap().write().await;
        Ok(tokens
            .iter()
            .cloned()
            .map(|token| {
                let balance = ellipticoin_contracts::Token::get_balance(
                    &mut db,
                    address.clone().into(),
                    token.clone().into(),
                );

                let total_supply =
                    ellipticoin_contracts::Token::get_total_supply(&mut db, token.clone().into());
                let price = if token.0 == USD {
                    BASE_FACTOR
                } else {
                    let token_supply = ellipticoin_contracts::AMM::get_pool_supply_of_token(
                        &mut db,
                        token.clone().into(),
                    );
                    let base_token_supply =
                        ellipticoin_contracts::AMM::get_pool_supply_of_base_token(
                            &mut db,
                            token.clone().into(),
                        );
                    if token_supply == 0 {
                        0
                    } else {
                        base_token_supply * BASE_FACTOR / token_supply
                    }
                };

                Token {
                    address: token,
                    balance: balance.into(),
                    price: price.into(),
                    total_supply: total_supply.into(),
                }
            })
            .collect())
    }

    async fn liquidity_tokens(
        _context: &Context,
        tokens: Vec<Address>,
        address: Address,
    ) -> Result<Vec<LiquidityToken>, FieldError> {
        let mut db = DB.get().unwrap().write().await;
        Ok(tokens
            .iter()
            .cloned()
            .map(|token| {
                let liquidity_token = AMM::liquidity_token(token.clone().into());
                let balance = ellipticoin_contracts::Token::get_balance(
                    &mut db,
                    address.clone().into(),
                    liquidity_token.clone(),
                );
                let total_supply =
                    ellipticoin_contracts::Token::get_total_supply(&mut db, liquidity_token);
                let pool_supply_of_token =
                    AMM::get_pool_supply_of_token(&mut db, token.clone().into());
                let pool_supply_of_base_token =
                    AMM::get_pool_supply_of_base_token(&mut db, token.clone().into());

                LiquidityToken {
                    token_address: token,
                    balance: U64(balance),
                    total_supply: U64(total_supply),
                    pool_supply_of_token: U64(pool_supply_of_token),
                    pool_supply_of_base_token: U64(pool_supply_of_base_token),
                }
            })
            .collect())
    }

    async fn proposals(_context: &Context) -> Vec<Proposal> {
        let mut db = DB.get().unwrap().write().await;
        let proposals = Governance::get_proposals(&mut db);
        proposals
            .iter()
            .cloned()
            .map(|proposal: governance::Proposal| Proposal {
                id: U64(proposal.id),
                proposer: Address(proposal.proposer),
                title: proposal.title,
                subtitle: proposal.subtitle,
                content: proposal.content,
                actions: proposal
                    .actions
                    .iter()
                    .cloned()
                    .map(|action| serde_cbor::to_vec(&action).unwrap().into())
                    .collect(),
                votes: proposal
                    .votes
                    .iter()
                    .map(|(address, vote)| {
                        let balance = ellipticoin_contracts::Token::get_balance(
                            &mut db,
                            address.clone().into(),
                            MS,
                        );
                        Vote {
                            address: (*address).into(),
                            yes: matches!(vote, ellipticoin_contracts::governance::Vote::For),
                            balance: U64(balance),
                        }
                    })
                    .collect(),
            })
            .collect()
    }

    async fn block_number(_context: &Context) -> Option<U64> {
        let mut db = DB.get().unwrap().write().await;
        let block_number = System::get_block_number(&mut db);
        Some(block_number.into())
    }

    async fn issuance_rewards(_context: &Context, address: Bytes) -> Result<U64, FieldError> {
        let mut db = DB.get().unwrap().write().await;
        let issuance_rewards = Ellipticoin::get_issuance_rewards(
            &mut db,
            address
                .0
                .try_into()
                .map_err(|_| anyhow!("Invalid address"))?,
        );
        Ok(U64(issuance_rewards))
    }

    async fn pending_redeem_requests(
        _context: &Context,
        address: Bytes,
    ) -> Result<Vec<RedeemRequest>, FieldError> {
        let address = <[u8; 20]>::try_from(address.0).map_err(|_| anyhow!("Invalid Address"))?;
        let mut db = DB.get().unwrap().write().await;
        let pending_redeem_requests = Bridge::get_pending_redeem_requests(&mut db);
        Ok(pending_redeem_requests
            .iter()
            .filter(|bridge::RedeemRequest { sender, .. }| *sender == address)
            .map(
                |bridge::RedeemRequest {
                     id,
                     sender,
                     token,
                     amount,
                     expiration_block_number,
                     signature,
                 }| RedeemRequest {
                    id: (*id).into(),
                    sender: (*sender).into(),
                    token: (*token).into(),
                    amount: (*amount).into(),
                    expiration_block_number: (*expiration_block_number).unwrap().into(),
                    signature: signature.as_ref().unwrap().to_vec().into(),
                },
            )
            .collect())
    }

    async fn next_transaction_number(
        _context: &Context,
        address: Bytes,
    ) -> Result<U64, FieldError> {
        let address = <[u8; 20]>::try_from(address.0).map_err(|_| anyhow!("Invalid Address"))?;
        let mut db = DB.get().unwrap().write().await;
        Ok(U64(System::get_next_transaction_number(&mut db, address)))
    }
}
