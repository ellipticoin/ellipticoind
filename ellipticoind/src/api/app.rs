use super::State;
use crate::api::{addresses, blocks, memory, transactions, storage};
use tide::middleware::Cors;

pub fn app(state: State) -> tide::server::Server<State> {
    let mut app = tide::with_state(state);
    app.middleware(Cors::new());
    app.at("/blocks").get(blocks::index);
    app.at("/blocks/:block_hash").get(blocks::show);
    app.at("/transactions/:transaction_hash")
        .get(transactions::show);
    app.at("/transactions").post(transactions::create);
    app.at("/memory/:key").get(memory::show);
    app.at("/storage/:key").get(storage::show);
    app.at("/addresses/:address").get(addresses::show);
    app
}
