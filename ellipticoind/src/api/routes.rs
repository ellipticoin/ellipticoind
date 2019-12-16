use crate::api::API;
use warp::http::header::{HeaderMap, HeaderValue};
use warp::Filter;

pub fn routes(api: API) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    let api2 = api.clone();
    let api3 = api.clone();
    let api4 = api.clone();
    let api5 = api.clone();
    let api6 = api.clone();
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/cbor"));

    let blocks = warp::path("websocket")
        .and(warp::ws())
        .and(warp::any().map(move || api.clone()))
        .map(|ws: warp::ws::Ws, mut api: API| {
            ws.on_upgrade(async move |socket| api.user_connected(socket).await)
        });
    let blocks_index = warp::path("blocks")
        .and(warp::any().map(move || api2.clone()))
        .and(warp::query())
        .map(super::blocks::blocks_index);
    let memory = warp::path("memory")
        .and(warp::any().map(move || api3.clone()))
        .and(warp::path::param())
        .map(super::memory::get_memory);
    let transactions_show = warp::path("transactions")
        .and(warp::any().map(move || api4.clone()))
        .and(warp::path::param())
        .map(super::transactions::show);
    let transactions = warp::post()
        .and(warp::path("transactions"))
        .and(warp::any().map(move || api5.clone()))
        .and(warp::body::cbor())
        .map(|api, transaction| super::transactions::create(api, transaction));

    let addresses_show = warp::path("addresses")
        .and(warp::any().map(move || api6.clone()))
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
