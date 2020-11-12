use super::graphql::handle_graphql;
use crate::api::{blocks, API};
use tide::sse;

impl API {
    pub fn routes(&mut self) {
        self.app
            .at("/static")
            .serve_dir("ellipticoind/static")
            .unwrap();
        self.app.at("/").get(sse::endpoint(blocks::broadcaster));
        self.app.at("/").post(handle_graphql);
    }
}
