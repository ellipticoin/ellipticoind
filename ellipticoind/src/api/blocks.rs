use crate::api::API;
use crate::diesel::query_dsl::limit_dsl::LimitDsl;
use crate::diesel::query_dsl::methods::OrderDsl;
use crate::diesel::ExpressionMethods;
use crate::diesel::GroupedBy;
use crate::diesel::RunQueryDsl;
use crate::models;
use crate::schema::blocks;
use diesel::BelongingToDsl;
use serde::Deserialize;
use warp::reply::Response;

#[derive(Deserialize)]
pub struct BlocksQuery {
    limit: i64,
}

pub fn blocks_index(api: API, query: BlocksQuery) -> Response {
    let con = api.db.get().unwrap();
    let blocks = blocks::dsl::blocks
        .order(crate::schema::blocks::dsl::number.desc())
        .limit(query.limit)
        .load::<models::Block>(&con)
        .unwrap();
    let transactions = models::Transaction::belonging_to(&blocks)
        .load::<models::Transaction>(&con)
        .unwrap()
        .grouped_by(&blocks);

    let blocks_response = blocks
        .into_iter()
        .zip(transactions)
        .map(|(block, transactions)| crate::api::Block::from((&block, &transactions)))
        .collect::<Vec<crate::api::Block>>();
    Response::new(serde_cbor::to_vec(&blocks_response).unwrap().into())
}
