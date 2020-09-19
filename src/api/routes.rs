use super::graphql::handle_graphql;
use crate::api::{blocks, memory, storage, API};
use tide::sse;

impl API {
    pub fn routes(&mut self) {
        self.app.at("/").get(sse::endpoint(blocks::broadcaster));
        self.app.at("/").post(handle_graphql);
        self.app.at("/memory/:contract/:key").get(memory::show);
        self.app.at("/storage/:contract/:key").get(storage::show);
    }
}
