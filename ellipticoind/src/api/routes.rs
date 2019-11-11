use crate::api::API;
use warp::http::header::{HeaderMap, HeaderValue};
use warp::Filter;

pub fn routes(api: API) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    let api2 = api.clone();
    let api3 = api.clone();
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/cbor"));

    let blocks = warp::path("blocks")
        .and(warp::ws())
        .and(warp::any().map(move || api.clone()))
        .map(|ws: warp::ws::Ws, mut api: API| {
            ws.on_upgrade(async move |socket| api.user_connected(socket).await)
        });
    let block_number = warp::path("block_number").map(super::blocks::block_number);
    let memory = warp::path("memory")
        .and(warp::any().map(move || api2.clone()))
        .and(warp::path::param())
        .map(super::memory::get_memory);
    let transactions = warp::post()
        .and(warp::path("transactions"))
        .and(warp::any().map(move || api3.clone()))
        .and(warp::body::cbor())
        .map(|api, transaction| super::transactions::create(api, transaction));

    memory
        .or(transactions)
        .or(block_number)
        .or(blocks)
        .with(warp::reply::with::headers(headers))
        .boxed()
}
