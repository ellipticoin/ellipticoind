use crate::system_contracts::ellipticoin::get_issuance_rewards;
use crate::{
    api::{graphql::Context, types::*},
    config::get_pg_connection,
    diesel::{BelongingToDsl, RunQueryDsl},
    models,
    models::transaction::next_nonce,
    schema::{blocks, blocks::columns::number, transactions},
    system_contracts::{api::ReadOnlyAPI, exchange, token},
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};

pub struct QueryRoot;
#[juniper::graphql_object(
    Context = Context,
)]
impl QueryRoot {
    async fn tokens(context: &Context, token_ids: Vec<TokenId>, address: Bytes) -> Vec<Token> {
        let mut api = ReadOnlyAPI::new(context.rocksdb.clone(), context.redis_pool.get().unwrap());
        token_ids
            .iter()
            .cloned()
            .map(|TokenId { id, issuer }| {
                let balance = token::get_balance(
                    &mut api,
                    ellipticoin::Token {
                        issuer: issuer.as_str().into(),
                        id: id.0.clone().into(),
                    },
                    address.0.clone().into(),
                );
                let total_supply = token::get_total_supply(
                    &mut api,
                    ellipticoin::Token {
                        issuer: issuer.as_str().into(),
                        id: id.0.clone().into(),
                    },
                );
                let price = exchange::price(
                    &mut api,
                    ellipticoin::Token {
                        issuer: issuer.as_str().into(),
                        id: id.0.clone().into(),
                    },
                );

                Token {
                    issuer: issuer.as_str().into(),
                    id: id.clone().into(),
                    balance: U64(balance),
                    price: U64(price),
                    total_supply: U64(total_supply),
                }
            })
            .collect()
    }

    async fn liquidity_tokens(
        context: &Context,
        token_ids: Vec<TokenId>,
        address: Bytes,
    ) -> Vec<LiquidityToken> {
        let mut api = ReadOnlyAPI::new(context.rocksdb.clone(), context.redis_pool.get().unwrap());
        token_ids
            .iter()
            .cloned()
            .map(|token| {
                let issuer = token.clone().issuer;
                let id = token.clone().id;
                let token = ellipticoin::Token::from(token);
                let liquidity_token = exchange::liquidity_token(token.clone());
                let balance =
                    token::get_balance(&mut api, liquidity_token.clone(), address.0.clone().into());
                let share_of_pool =
                    exchange::share_of_pool(&mut api, token.clone(), address.0.clone().into());
                let price = exchange::get_price(&mut api, token).unwrap_or(0);

                LiquidityToken {
                    issuer,
                    id,
                    balance: U64(balance),
                    price: U64(price),

                    share_of_pool: U32(share_of_pool),
                }
            })
            .collect()
    }

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

    async fn issuance_rewards(context: &Context, address: Bytes) -> Option<U64> {
        let mut api = ReadOnlyAPI::new(context.rocksdb.clone(), context.redis_pool.get().unwrap());
        let issuance_rewards = get_issuance_rewards(&mut api, <Vec<u8>>::from(address).into());
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

    async fn next_nonce(_context: &Context, address: Bytes) -> U32 {
        U32(next_nonce(address.0))
    }
}
