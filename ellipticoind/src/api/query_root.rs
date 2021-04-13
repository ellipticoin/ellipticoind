use crate::{
    api::{
        graphql::Context,
        types::{self, *},
    },
    aquire_db_read_lock,
    constants::DB,
};
use anyhow::anyhow;
use ellipticoin_contracts::{
    bridge, governance, order_book, Bridge, Ellipticoin, Governance, OrderBook, System, AMM,
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
        let mut db = aquire_db_read_lock!();
        Ok(tokens
            .iter()
            .cloned()
            .map(|token| {
                let balance = ellipticoin_contracts::Token::get_underlying_balance(
                    &mut db,
                    address.clone().into(),
                    token.clone().into(),
                );
                let interest_rate =
                    ellipticoin_contracts::Token::get_interest_rate(&mut db, token.clone().into());
                let price = ellipticoin_contracts::Token::get_underlying_price(
                    &mut db,
                    token.clone().into(),
                );

                let total_supply = ellipticoin_contracts::Token::get_underlying_total_supply(
                    &mut db,
                    token.clone().into(),
                );

                Token {
                    address: token,
                    interest_rate: interest_rate.map(|interest_rate| interest_rate.into()),
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
        let mut db = aquire_db_read_lock!();
        Ok(tokens
            .iter()
            .cloned()
            .map(|token| {
                let balance =
                    AMM::get_balance(&mut db, address.clone().into(), token.clone().into());
                let total_supply = AMM::get_total_supply(&mut db, token.clone().into());
                let pool_supply_of_token =
                    AMM::get_pool_supply_of_token(&mut db, token.clone().into());
                let underlying_pool_supply_of_base_token =
                    AMM::get_underlying_pool_supply_of_base_token(&mut db, token.clone().into());

                LiquidityToken {
                    token_address: token,
                    balance: U64(balance),
                    total_supply: U64(total_supply),
                    pool_supply_of_token: U64(pool_supply_of_token),
                    pool_supply_of_base_token: U64(underlying_pool_supply_of_base_token),
                }
            })
            .collect())
    }

    async fn orders(_context: &Context) -> Vec<Order> {
        let mut db = aquire_db_read_lock!();
        let orders = OrderBook::get_orders(&mut db);
        orders
            .iter()
            .cloned()
            .map(|order: order_book::Order| {
                let price = order.get_underlying_price(&mut db);
                let amount = order.get_underlying_amount(&mut db);

                return Order {
                    order_type: format!("{:?}", order.order_type),
                    id: U64(order.id),
                    token: order.token.into(),
                    amount: U64(amount),
                    price: U64(price),
                };
            })
            .collect()
    }

    async fn proposals(_context: &Context) -> Vec<Proposal> {
        let mut db = aquire_db_read_lock!();
        let proposals = Governance::get_proposals(&mut db);
        proposals
            .iter()
            .cloned()
            .map(|proposal: governance::Proposal| Proposal {
                id: U64(proposal.id as u64),
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
                    .map(|vote| Vote {
                        voter: vote.voter.into(),
                        choice: format!("{:?}", vote.choice),
                        weight: U64(vote.weight),
                    })
                    .collect(),
                result: proposal.result.map(|result| format!("{:?}", result)),
            })
            .collect()
    }

    async fn block_number(_context: &Context) -> Option<U64> {
        let mut db = aquire_db_read_lock!();
        let block_number = System::get_block_number(&mut db);
        Some(block_number.into())
    }

    async fn issuance_rewards(_context: &Context, address: Bytes) -> Result<U64, FieldError> {
        let mut db = aquire_db_read_lock!();
        let issuance_rewards = Ellipticoin::get_issuance_rewards(
            &mut db,
            ellipticoin_types::Address(
                address
                    .0
                    .try_into()
                    .map_err(|_| anyhow!("Invalid address"))?,
            ),
        );
        Ok(U64(issuance_rewards))
    }

    async fn pending_redeem_requests(
        _context: &Context,
        address: Bytes,
    ) -> Result<Vec<RedeemRequest>, FieldError> {
        let address = ellipticoin_types::Address(
            <[u8; 20]>::try_from(address.0).map_err(|_| anyhow!("Invalid Address"))?,
        );
        let mut db = aquire_db_read_lock!();
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
        let address = ellipticoin_types::Address(
            <[u8; 20]>::try_from(address.0).map_err(|_| anyhow!("Invalid Address"))?,
        );
        let mut db = aquire_db_read_lock!();
        Ok(U64(System::get_next_transaction_number(&mut db, address)))
    }
}
