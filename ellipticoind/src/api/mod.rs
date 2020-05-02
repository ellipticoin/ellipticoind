use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use futures::channel::mpsc::UnboundedSender;
use std::sync::{Arc, Mutex};
use tungstenite::protocol::Message;
use futures::channel::mpsc;
use network::serde::Serialize;
pub use views::Block;
mod addresses;
pub mod app;
mod blocks;
mod memory;
mod transactions;
mod views;
pub mod websocket;

pub struct State {
    pub websockets: Arc<Mutex<Vec<UnboundedSender<Message>>>>,
    pub redis: vm::redis::Client,
    pub rocksdb: Arc<rocksdb::DB>,
    pub db: Pool<ConnectionManager<PgConnection>>,
    pub network_sender: mpsc::Sender<crate::network::Message>,
}

impl State {
    pub fn new(
        redis: vm::redis::Client,
        rocksdb: Arc<rocksdb::DB>,
        db: Pool<ConnectionManager<PgConnection>>,
        network_sender: mpsc::Sender<crate::network::Message>,
    ) -> Self {
        Self {
            websockets: Arc::new(Mutex::new(Vec::new())),
            redis,
            rocksdb,
            db,
            network_sender,
        }
    }
}
