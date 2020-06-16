use super::ApiState;
use crate::api::{addresses, blocks, memory, state, storage, transactions};
use tide::middleware::Cors;

pub fn app(state: ApiState) -> tide::server::Server<ApiState> {
    let mut app = tide::with_state(state);
    app.middleware(Cors::new());
    app.at("/p2p/blocks").post(blocks::create);
    app.at("/blocks").get(blocks::index);
    app.at("/blocks/:block_hash").get(blocks::show);
    app.at("/transactions/:transaction_hash")
        .get(transactions::show);
    app.at("/transactions").post(transactions::broadcast);
    app.at("/p2p/transactions").post(transactions::create);
    app.at("/memory/:key").get(memory::show);
    app.at("/storage/:key").get(storage::show);
    app.at("/state").get(state::show);
    app.at("/addresses/:address").get(addresses::show);
    app
}
