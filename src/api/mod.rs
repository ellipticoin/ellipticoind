use crate::{models, models::Transaction, transaction};
use async_std::sync::{channel, Receiver, Sender};
use broadcaster::BroadcastChannel;
use futures::channel::oneshot;
pub use futures::stream::StreamExt;

use std::net::SocketAddr;

mod addresses;
pub mod app;
mod blocks;
mod helpers;
mod memory;
mod middleware;
mod routes;
mod storage;
mod transactions;
pub mod views;
pub struct API {
    pub app: tide::Server<State>,
}

pub enum Message {
    Transaction(transaction::Transaction, oneshot::Sender<Transaction>),
    Block((models::Block, Vec<Transaction>)),
}

impl API {
    pub fn new() -> (BroadcastChannel<Vec<u8>>, Receiver<Message>, Self) {
        let (sender, receiver) = channel(1);
        let new_block_broacaster = BroadcastChannel::new();
        let state = State::new(sender, new_block_broacaster.clone());
        // tide::log::with_level(tide::log::LevelFilter::Trace);
        let mut app = tide::with_state(state);
        app.with(tide::log::LogMiddleware::new());

        let mut api = Self { app };
        api.middleware();
        api.routes();
        (new_block_broacaster, receiver, api)
    }

    pub async fn listen(self, socket: SocketAddr) {
        self.app.listen(socket).await.unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub sender: Sender<Message>,
    pub new_block_broacaster: BroadcastChannel<Vec<u8>>,
}
impl State {
    pub fn new(sender: Sender<Message>, new_block_broacaster: BroadcastChannel<Vec<u8>>) -> Self {
        Self {
            sender,
            new_block_broacaster,
        }
    }
}
