extern crate juniper;
use crate::{
    api::{mutations::Mutations, query_root::QueryRoot, State},
    helpers::current_miner,
};
use async_std::sync::Sender;
use juniper::{EmptySubscription, Variables};
use serde_json::json;
use std::{fmt, sync::Arc};
use tide::{http::StatusCode, Body, Request, Response};

impl juniper::Context for State {}

pub struct Context {
    pub rocksdb: Arc<rocksdb::DB>,
    pub redis_pool: crate::types::redis::Pool,
    pub sender: Sender<crate::api::Message>,
}
impl juniper::Context for Context {}

#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

pub type Schema = juniper::RootNode<'static, QueryRoot, Mutations, EmptySubscription<Context>>;
pub async fn handle_graphql(mut request: Request<State>) -> tide::Result {
    let current_miner = current_miner().await;
    // if !current_miner.address.eq(&verification_key()) {
    //     return proxy_post(&mut request, current_miner.host).await;
    // }

    let ctx = Context {
        rocksdb: crate::config::ROCKSDB.clone(),
        redis_pool: crate::config::REDIS_POOL.clone(),
        sender: request.state().sender.clone(),
    };

    let body_json = request
        .body_json::<std::collections::HashMap<String, serde_json::value::Value>>()
        .await
        .unwrap();
    let query: String = body_json
        .get("query")
        .unwrap_or(&serde_json::Value::Null)
        .as_str()
        .unwrap_or("")
        .to_string();
    let variables: Variables = serde_json::value::from_value(
        body_json
            .get("variables")
            .unwrap_or(&serde_json::value::Value::Null)
            .clone(),
    )?;

    let (res, errors) = juniper::execute(
        &query,
        None,
        &Schema::new(QueryRoot, Mutations, EmptySubscription::new()),
        &variables,
        &ctx,
    )
    .await
    .unwrap();
    // .map_err(|e| http::Error::from_str(StatusCode::BadRequest, e.to_string()))?;

    Ok(Response::builder(StatusCode::Ok)
        .body(Body::from_json(&json!({
        "data": res,
        "errors": errors,
        }))?)
        .build())
}
