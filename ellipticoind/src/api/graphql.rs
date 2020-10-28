extern crate juniper;
use crate::api::{mutations::Mutations, query_root::QueryRoot};
use juniper::{graphql_value, EmptySubscription, Variables};
use serde_json::json;
use std::{fmt, sync::Arc};
use tide::{http::StatusCode, Body, Request, Response};

pub struct Context {
    pub rocksdb: Arc<rocksdb::DB>,
    pub redis_pool: crate::types::redis::Pool,
}
impl juniper::Context for Context {}

#[derive(Debug)]
pub struct Error(pub String);

impl juniper::IntoFieldError for Error {
    fn into_field_error(self) -> juniper::FieldError {
        juniper::FieldError::new(
            self.to_string(),
            graphql_value!({
                "type": "Error"
            }),
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

pub type Schema = juniper::RootNode<'static, QueryRoot, Mutations, EmptySubscription<Context>>;
pub async fn handle_graphql(mut request: Request<()>) -> tide::Result {
    let ctx = Context {
        rocksdb: crate::config::ROCKSDB.clone(),
        redis_pool: crate::config::REDIS_POOL.clone(),
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
    .map_err(|e| http::Error::from_str(StatusCode::BadRequest, e.to_string()))?;
    // .unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .body(Body::from_json(&json!({
        "data": res,
        "errors": errors,
        }))?)
        .build())
}
