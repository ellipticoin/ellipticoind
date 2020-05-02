use super::State;
use crate::api::views::Block;
use crate::diesel::ExpressionMethods;
use crate::diesel::GroupedBy;
use crate::diesel::OptionalExtension;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use crate::models;
use crate::schema::blocks;
use crate::schema::blocks::columns::number;
use diesel::BelongingToDsl;
use http_service::Body;
use serde::Deserialize;
use serde_cbor::ser::to_vec;
use tide::Response;
use serde::Serialize;

#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<i64>,
}

pub async fn index(req: tide::Request<State>) -> Response {
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
        .map(|(block, transactions)| Block::from((&block, &transactions)))
        .collect::<Vec<Block>>();
    Response::new(200)
        .body(Body::from(to_vec(&blocks_response).unwrap()))
        .set_header("Content-type", "application/cors")
}

pub async fn show(req: tide::Request<State>) -> Response {
    let block_param: String = req.param("block_hash").unwrap();
    let con = req.state().db.get().unwrap();
    let block = match block_param.parse::<i64>() {
        Ok(block_number) => blocks::dsl::blocks
            .filter(number.eq(block_number))
            .first::<models::Block>(&con)
            .optional()
            .unwrap(),
        Err(_) => blocks::dsl::blocks
            .find(base64::decode_config(&block_param, base64::URL_SAFE).unwrap())
            .first::<models::Block>(&con)
            .optional()
            .unwrap(),
    };

    if let Some(block) = block {
        let transactions = models::Transaction::belonging_to(&block)
            .load::<models::Transaction>(&con)
            .unwrap();
        let blocks_response = Block::from((&block, &transactions));
        Response::new(200)
            .body(Body::from(to_vec(&blocks_response).unwrap()))
            .set_header("Content-type", "application/cors")
    } else {
        Response::new(404)
    }
}
