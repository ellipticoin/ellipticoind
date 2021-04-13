use crate::{
    api::{
<<<<<<< HEAD
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
=======
        graphql::{Context, Error},
        types::*,
    },
    config::get_pg_connection,
    diesel::{BelongingToDsl, RunQueryDsl},
    models,
    models::transaction::next_nonce,
    schema::{blocks, blocks::columns::number, transactions},
    state::IN_MEMORY_STATE,
    system_contracts::{
        api::InMemoryAPI, ellipticoin::get_issuance_rewards, exchange,
        exchange::constants::BASE_TOKEN, token, token::BASE_FACTOR,
    },
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use ellipticoin::Address;
use std::convert::TryInto;
>>>>>>> master

pub struct QueryRoot;
#[juniper::graphql_object(
    Context = Context,
)]
impl QueryRoot {
<<<<<<< HEAD
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

                let total_supply =
                    ellipticoin_contracts::Token::get_underlying_total_supply(&mut db, token.clone().into());

                Token {
                    address: token,
                    interest_rate: interest_rate.map(|interest_rate| interest_rate.into()),
                    balance: balance.into(),
                    price: price.into(),
                    total_supply: total_supply.into(),
=======
    async fn tokens(
        _context: &Context,
        token_ids: Vec<TokenId>,
        address: Bytes,
    ) -> Result<Vec<Token>, Error> {
        let address: Address = address
            .0
            .clone()
            .try_into()
            .map_err(|e: Box<wasm_rpc::error::Error>| Error(e.to_string()))?;
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut api = InMemoryAPI::new(&mut state, None);
        Ok(token_ids
            .iter()
            .cloned()
            .map(|TokenId { id, issuer }| {
                let balance = token::get_balance(
                    &mut api,
                    ellipticoin::Token {
                        issuer: issuer.as_str().into(),
                        id: id.0.clone().into(),
                    },
                    address.clone(),
                );
                let total_supply = token::get_total_supply(
                    &mut api,
                    ellipticoin::Token {
                        issuer: issuer.as_str().into(),
                        id: id.0.clone().into(),
                    },
                );
                let price = if Vec::from(id.clone()) == BASE_TOKEN.clone().id.into_vec() {
                    BASE_FACTOR
                } else {
                    exchange::get_price(
                        &mut api,
                        ellipticoin::Token {
                            issuer: issuer.as_str().into(),
                            id: id.0.clone().into(),
                        },
                    )
                    .unwrap_or(0)
                };

                Token {
                    issuer: issuer.as_str().into(),
                    id: id.clone().into(),
                    balance: U64(balance),
                    price: U64(price),
                    total_supply: U64(total_supply),
>>>>>>> master
                }
            })
            .collect())
    }

    async fn liquidity_tokens(
        _context: &Context,
<<<<<<< HEAD
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
=======
        token_ids: Vec<TokenId>,
        address: Bytes,
    ) -> Result<Vec<LiquidityToken>, Error> {
        let address: Address = address
            .0
            .clone()
            .try_into()
            .map_err(|e: Box<wasm_rpc::error::Error>| Error(e.to_string()))?;
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut api = InMemoryAPI::new(&mut state, None);
        Ok(token_ids
            .iter()
            .cloned()
            .map(|token| {
                let issuer = token.clone().issuer;
                let id = token.clone().id;
                let token = ellipticoin::Token::from(token);
                let liquidity_token = exchange::liquidity_token(token.clone());
                let balance =
                    token::get_balance(&mut api, liquidity_token.clone(), address.clone());
                let price = exchange::get_price(&mut api, token.clone()).unwrap_or(0);
                let total_supply = token::get_total_supply(&mut api, liquidity_token.clone());
                let pool_supply_of_token =
                    exchange::get_pool_supply_of_token(&mut api, token.clone());
                let pool_supply_of_base_token =
                    exchange::get_pool_supply_of_base_token(&mut api, token.clone());

                LiquidityToken {
                    issuer,
                    id,
                    balance: U64(balance),
                    price: U64(price),
                    total_supply: U64(total_supply),
                    pool_supply_of_token: U64(pool_supply_of_token),
                    pool_supply_of_base_token: U64(pool_supply_of_base_token),
>>>>>>> master
                }
            })
            .collect())
    }

<<<<<<< HEAD
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
                }
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
=======
    async fn block(_context: &Context, block_number: U32) -> Option<Block> {
        let con = get_pg_connection();
        blocks::dsl::blocks
            .filter(number.eq(block_number.0 as i32))
            .first::<models::Block>(&con)
            .optional()
            .ok()
            .flatten()
            .map(|block| {
                let transactions = (models::Transaction::belonging_to(&block)
                    .order(transactions::dsl::position.asc())
                    .load::<models::Transaction>(&con))
                .unwrap_or(vec![]);
                Block::from((block, transactions))
            })
    }

    async fn current_block_number(_context: &Context) -> Option<U32> {
        let block_number = models::Block::current_block_number();
        Some(block_number.into())
    }

    async fn issuance_rewards(_context: &Context, address: Bytes) -> Option<U64> {
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut api = InMemoryAPI::new(&mut state, None);
        let issuance_rewards =
            get_issuance_rewards(&mut api, <Vec<u8>>::from(address).try_into().ok()?);
        Some(issuance_rewards.into())
    }

    async fn current_block(_context: &Context) -> Block {
        let con = get_pg_connection();
        blocks::dsl::blocks
            .filter(blocks::sealed.eq(true))
            .order_by(number.desc())
            .first::<models::Block>(&con)
            .map(|block| {
                let transactions = (models::Transaction::belonging_to(&block)
                    .order(transactions::dsl::position.asc())
                    .load::<models::Transaction>(&con))
                .unwrap_or(vec![]);
                Block::from((block, transactions))
            })
            .unwrap()
    }

    async fn transaction(_context: &Context, transaction_id: U32) -> Option<Transaction> {
        let con = get_pg_connection();
        transactions::dsl::transactions
            .find(transaction_id.0 as i32)
            .first::<models::Transaction>(&con)
            .optional()
            .ok()
            .flatten()
            .map(Transaction::from)
    }

    async fn transactions_by_contract_function(
        _context: &Context,
        sender_address: Bytes,
        contract_name: String,
        function_name: String,
        page: U64,
        page_size: U64,
    ) -> Vec<Transaction> {
        let con = get_pg_connection();
        transactions::dsl::transactions
            .filter(transactions::sender.eq(<Vec<u8>>::from(sender_address)))
            .filter(transactions::contract.eq(contract_name))
            .filter(transactions::function.eq(function_name))
            .order_by(transactions::id.desc())
            .limit(page_size.0.clone() as i64)
            .offset((page.0 as i64) * (page_size.0 as i64))
            .load::<models::Transaction>(&con)
            .expect("Error loading exit transactions for")
            .into_iter()
            .map(Transaction::from)
            .collect()
    }

    async fn next_nonce(_context: &Context, address: Bytes) -> U32 {
        U32(next_nonce(address.0))
>>>>>>> master
    }
}
