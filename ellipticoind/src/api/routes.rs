use crate::api::API;
use warp::http::header::{HeaderMap, HeaderValue};
use warp::Filter;

pub fn routes(api: API) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/cbor"));

    let any = warp::any().map(move || api.clone());
    let blocks = warp::path("websocket")
        .and(warp::ws())
        .and(any.clone())
        .map(|ws: warp::ws::Ws, mut api: API| {
            ws.on_upgrade(async move |socket| api.user_connected(socket).await)
        });
    let blocks_index = any
        .clone()
        .and(warp::path("blocks"))
        .and(warp::query())
        .map(super::blocks::blocks_index);
    let memory = any
        .clone()
        .and(warp::path("memory"))
        .and(warp::path::param())
        .map(super::memory::get_memory);
    let transactions_show = any
        .clone()
        .and(warp::path("transactions"))
        .and(warp::path::param())
        .map(super::transactions::show);
    let transactions = any
        .clone()
        .and(warp::post())
        .and(warp::path("transactions"))
        .and(warp::body::cbor())
        .map(|api, transaction| super::transactions::create(api, transaction));

    let addresses_show = any
        .clone()
        .and(warp::path("addresses"))
        .and(warp::path::param())
        .map(super::addresses::show);

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[warp::http::Method::GET]);

    addresses_show
        .or(memory)
        .or(transactions)
        .or(transactions_show)
        .or(blocks_index)
        .or(blocks)
        .with(cors)
        .with(warp::reply::with::headers(headers))
        .boxed()
}
