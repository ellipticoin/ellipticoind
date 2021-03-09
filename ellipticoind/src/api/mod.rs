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
use async_std::task::spawn;
use std::net::SocketAddr;
use tide::listener::Listener;
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

pub async fn start(socket: SocketAddr) {
    let api = API::new();
    let mut listener = api.app.bind(socket).await.unwrap();
    for info in listener.info().iter() {
        println!("Server listening on {}", info);
    }
    spawn(async move { listener.accept().await.unwrap() });
}
