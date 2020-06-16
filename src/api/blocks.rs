use super::ApiState;
use crate::api::views::Block;
use crate::diesel::ExpressionMethods;
use crate::diesel::GroupedBy;
use crate::diesel::OptionalExtension;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use crate::models;
use crate::network::Message;
use crate::schema::blocks;
use crate::schema::blocks::columns::number;
use diesel::BelongingToDsl;
use http_service::Body;
use serde::Deserialize;

use serde_cbor::ser::to_vec;
use tide::Response;

#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<i64>,
}

pub async fn create(mut req: tide::Request<ApiState>) -> Response {
    let block_bytes = req.body_bytes().await.unwrap();
    let block_view: crate::api::views::Block = serde_cbor::value::from_value(
        serde_cbor::from_slice::<serde_cbor::Value>(&block_bytes).unwrap(),
    )
    .unwrap();
    let (block, mut transactions) = block_view.into();
    transactions.iter_mut().for_each(|transaction| {
        transaction.set_hash();
        transaction.block_hash = block.hash.clone();
    });
    let mut ordered_transactions = transactions.clone();
    ordered_transactions.sort_by(|a, b| {
        if a.function == "start_mining" {
            std::cmp::Ordering::Less
        } else if b.function == "start_mining" {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });

    let miner_sender = &req.state().miner_sender;
    miner_sender
        .send(Message::Block((block, transactions)))
        .await;
    Response::new(201)
}

pub async fn index(req: tide::Request<ApiState>) -> Response {
    let query = req.query::<QueryParams>().unwrap();
    let con = req.state().db.get().unwrap();
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
    Response::new(200)
        .body(Body::from(to_vec(&blocks_response).unwrap()))
        .set_header("Content-type", "application/cors")
}

pub async fn show(req: tide::Request<ApiState>) -> Response {
    let block_param: String = req.param("block_hash").unwrap_or("".to_string());
    let con = req.state().db.get().unwrap();
    let block = match block_param.parse::<i64>() {
        Ok(block_number) => {
            if let Ok(block) = blocks::dsl::blocks
                .filter(number.eq(block_number))
                .first::<models::Block>(&con)
                .optional()
            {
                block
            } else {
                return Response::new(404);
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
                return Response::new(404);
            }
        }
    };

    if let Some(block) = block {
        let transactions = models::Transaction::belonging_to(&block)
            .load::<models::Transaction>(&con)
            .unwrap();
        let blocks_response = Block::from((block, transactions));
        Response::new(200)
            .body(Body::from(to_vec(&blocks_response).unwrap()))
            .set_header("Content-type", "application/cors")
    } else {
        Response::new(404)
    }
}
