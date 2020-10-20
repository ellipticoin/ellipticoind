pub use futures::stream::StreamExt;
pub mod app;
mod blocks;
pub mod graphql;
mod helpers;
mod middleware;
mod mutations;
mod query_root;
mod routes;
mod types;
pub mod views;
pub struct API {
    pub app: tide::Server<()>,
}

impl API {
    pub fn new() -> Self {
        let mut app = tide::new();
        app.with(tide::log::LogMiddleware::new());

        let mut api = Self { app };
        api.middleware();
        api.routes();
        api
    }
}
