use super::{helpers::to_cbor_response, ApiState};
use crate::{
    api::{views, views::Block},
    config::get_pg_connection,
    diesel::{ExpressionMethods, GroupedBy, OptionalExtension, QueryDsl, RunQueryDsl},
    models,
    network::Message,
    schema::{blocks, blocks::columns::number},
};
use diesel::BelongingToDsl;
use serde::Deserialize;
use tide::{http::StatusCode, Response};

#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<i64>,
}

pub async fn create(mut req: tide::Request<ApiState>) -> tide::Result<tide::Response> {
    let block_bytes = req.body_bytes().await.unwrap();
    let block_view: Block = serde_cbor::from_slice::<views::Block>(&block_bytes)
        .unwrap()
        .into();
    let miner_sender = &req.state().miner_sender;
    miner_sender.send(Message::Block(block_view.into())).await;
    Ok(Response::new(StatusCode::Ok))
}

pub async fn index(req: tide::Request<ApiState>) -> tide::Result<Response> {
    let query = req.query::<QueryParams>().unwrap();
    let con = get_pg_connection();
    let blocks = blocks::dsl::blocks
        .order(blocks::dsl::number.desc())
        .limit(query.limit.unwrap())
        .load::<models::Block>(&con)
        .unwrap();
    let transactions = models::Transaction::belonging_to(&blocks)
        .load::<models::Transaction>(&con)
        .unwrap()
        .grouped_by(&blocks);
    let blocks_response = blocks
        .into_iter()
        .zip(transactions)
        .map(|(block, transactions)| Block::from((block, transactions)))
        .collect::<Vec<Block>>();
    Ok(to_cbor_response(&blocks_response))
}

pub async fn show(req: tide::Request<ApiState>) -> tide::Result<Response> {
    let block_param: String = req.param("block_hash").unwrap_or("".to_string());
    let con = get_pg_connection();
    let block = match block_param.parse::<i64>() {
        Ok(block_number) => {
            if let Ok(block) = blocks::dsl::blocks
                .filter(number.eq(block_number))
                .first::<models::Block>(&con)
                .optional()
            {
                block
            } else {
                return Ok(Response::new(404));
            }
        }
        Err(_) => {
            if let Ok(block) = blocks::dsl::blocks
                .find(base64::decode_config(&block_param, base64::URL_SAFE).unwrap_or(vec![]))
                .first::<models::Block>(&con)
                .optional()
            {
                block
            } else {
                return Ok(Response::new(404));
            }
        }
    };

    if let Some(block) = block {
        let transactions = models::Transaction::belonging_to(&block)
            .load::<models::Transaction>(&con)
            .unwrap();
        let blocks_response = Block::from((block, transactions));
        Ok(to_cbor_response(&blocks_response))
    } else {
        Ok(Response::new(404))
    }
}
