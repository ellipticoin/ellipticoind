use crate::api::{addresses, blocks, memory, state, storage, transactions, API};
use tide::sse;

impl API {
    pub fn routes(&mut self) {
        self.app.at("/").get(sse::endpoint(blocks::broadcaster));

        self.app.at("/blocks").post(blocks::create);
        self.app.at("/blocks").get(blocks::index);
        self.app.at("/blocks/:block_hash").get(blocks::show);
        self.app
            .at("/transactions/:transaction_hash")
            .get(transactions::show);
        self.app.at("/transactions").post(transactions::create);
        self.app
            .at("/memory/:contract_owner/:contract_name/:key")
            .get(memory::show);
        self.app
            .at("/storage/:contract_owner/:contract_name/:key")
            .get(storage::show);
        self.app.at("/state").get(state::show);
        self.app.at("/addresses/:address").get(addresses::show);
    }
}
