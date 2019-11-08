use crate::api::API;
use warp::http::header::{HeaderMap, HeaderValue};
use warp::Filter;

static INDEX_HTML: &str = include_str!("../index.html");

pub fn routes(api: API) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/cbor"));

    let blocks = warp::path("blocks")
        .and(warp::ws())
        .and(warp::any().map(move || api.clone()))
        .map(|ws: warp::ws::Ws, mut api: API| {
            ws.on_upgrade(async move |socket| api.user_connected(socket).await)
        });
    let block_number = warp::path("block_number").map(super::blocks::block_number);
    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));
    let transactions = warp::post()
        .and(warp::path("transactions"))
        .and(warp::body::cbor())
        .map(|t| super::transactions::create(t));

    index
        .or(transactions)
        .or(block_number)
        .or(blocks)
        .with(warp::reply::with::headers(headers))
        .boxed()
}
