use crate::{models, models::Transaction, vm};
use async_std::sync::{channel, Receiver, Sender};
use futures::channel::{mpsc::UnboundedSender, oneshot};
pub use futures::stream::StreamExt;

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

mod addresses;
pub mod app;
mod blocks;
mod helpers;
mod memory;
mod middleware;
mod routes;
pub mod state;
mod storage;
mod transactions;
pub mod views;
pub mod websocket;
pub struct API {
    pub app: tide::Server<State>,
}

pub enum Message {
    Transaction(vm::Transaction, oneshot::Sender<Transaction>),
    Block((models::Block, Vec<Transaction>)),
}

impl API {
    pub fn new() -> (Receiver<Message>, Self) {
        let (sender, receiver) = channel(1);
        let state = State::new(sender);
        let app = tide::with_state(state);
        let mut api = Self { app };
        api.middleware();
        api.routes();
        (receiver, api)
    }

    pub async fn listen(self, socket: SocketAddr) {
        self.app.listen(socket).await.unwrap();
    }
}
pub struct State {
    pub websockets: Arc<Mutex<Vec<UnboundedSender<tungstenite::protocol::Message>>>>,
    pub sender: Sender<Message>,
}
impl State {
    pub fn new(sender: Sender<Message>) -> Self {
        Self {
            websockets: Arc::new(Mutex::new(Vec::new())),
            sender,
        }
    }
}
