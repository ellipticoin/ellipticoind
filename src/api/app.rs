use super::ApiState;
use crate::{
    api,
    api::{addresses, blocks, memory, state, storage, transactions},
};
use std::{future::Future, pin::Pin};
use tide::{security::CorsMiddleware, Next, Request, Result};

fn cbor_middleware<'a>(
    request: Request<api::ApiState>,
    next: Next<'a, api::ApiState>,
) -> Pin<Box<dyn Future<Output = Result> + Send + 'a>> {
    Box::pin(async {
        next.run(request).await.and_then(|mut response| {
            response.insert_header("Content-Type", "application/cbor");
            Ok(response)
        })
    })
}

pub fn app(state: ApiState) -> tide::Server<ApiState> {
    let mut app = tide::with_state(state);
    let cors_middleware = CorsMiddleware::new();
    app.middleware(cors_middleware);
    app.middleware(cbor_middleware);
    app.at("/blocks").post(blocks::create);
    app.at("/blocks").get(blocks::index);
    app.at("/blocks/:block_hash").get(blocks::show);
    app.at("/transactions/:transaction_hash")
        .get(transactions::show);
    app.at("/transactions").post(transactions::create);
    app.at("/memory/:contract_owner/:contract_name/:key")
        .get(memory::show);
    app.at("/storage/:contract_owner/:contract_name/:key")
        .get(storage::show);
    app.at("/state").get(state::show);
    app.at("/addresses/:address").get(addresses::show);
    app
}
