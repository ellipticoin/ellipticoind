use crate::{
    api::{
        graphql::Context,
        types::{self, *},
    },
    db::MemoryDB,
    state::IN_MEMORY_STATE,
};
use anyhow::anyhow;
use ellipticoin_contracts::{
    bridge,
    constants::{BASE_FACTOR, USD},
    Bridge, Ellipticoin, Exchange, System,
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
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut db = MemoryDB::new(&mut state);
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
                    let token_supply = ellipticoin_contracts::Exchange::get_pool_supply_of_token(
                        &mut db,
                        token.clone().into(),
                    );
                    let base_token_supply =
                        ellipticoin_contracts::Exchange::get_pool_supply_of_base_token(
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
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut db = MemoryDB::new(&mut state);
        Ok(tokens
            .iter()
            .cloned()
            .map(|token| {
                let liquidity_token = Exchange::liquidity_token(token.clone().into());
                let balance = ellipticoin_contracts::Token::get_balance(
                    &mut db,
                    address.clone().into(),
                    liquidity_token.clone(),
                );
                let total_supply =
                    ellipticoin_contracts::Token::get_total_supply(&mut db, liquidity_token);
                let pool_supply_of_token =
                    Exchange::get_pool_supply_of_token(&mut db, token.clone().into());
                let pool_supply_of_base_token =
                    Exchange::get_pool_supply_of_base_token(&mut db, token.clone().into());

                println!("total_supply:{}", total_supply);
                println!("balance:{}", balance);
                println!("pool_supply_of_token:{}", pool_supply_of_token);
                println!("pool_supply_of_base_token:{}", pool_supply_of_base_token);
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

    // async fn block(_context: &Context, block_number: U32) -> Option<Block> {
    //     let con = get_pg_connection();
    //     blocks::dsl::blocks
    //         .filter(number.eq(block_number.0 as i32))
    //         .first::<models::Block>(&con)
    //         .optional()
    //         .ok()
    //         .flatten()
    //         .map(|block| {
    //             let transactions = (models::Transaction::belonging_to(&block)
    //                 .order(transactions::dsl::position.asc())
    //                 .load::<models::Transaction>(&con))
    //             .unwrap_or(vec![]);
    //             Block::from((block, transactions))
    //         })
    // }
    //
    async fn block_number(_context: &Context) -> Option<U64> {
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut db = MemoryDB::new(&mut state);
        let block_number = System::get_block_number(&mut db);
        Some(block_number.into())
    }

    async fn issuance_rewards(_context: &Context, address: Bytes) -> Result<U64, FieldError> {
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut db = MemoryDB::new(&mut state);
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
        let mut state = IN_MEMORY_STATE.lock().await;
        let address = <[u8; 20]>::try_from(address.0).map_err(|_| anyhow!("Invalid Address"))?;
        let mut db = MemoryDB::new(&mut state);
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
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut db = MemoryDB::new(&mut state);
        Ok(U64(System::get_next_transaction_number(&mut db, address)))
    }
}
