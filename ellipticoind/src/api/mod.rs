use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use futures::channel::mpsc::UnboundedSender;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tungstenite::protocol::Message;
use network::Sender;
pub use views::Block;
mod addresses;
pub mod app;
mod blocks;
mod memory;
mod transactions;
mod views;
pub mod websocket;

#[derive(Clone)]
pub struct State {
    pub websockets: Arc<Mutex<Vec<UnboundedSender<Message>>>>,
    pub redis: vm::redis::Client,
    pub rocksdb: Arc<rocksdb::DB>,
    pub db: Pool<ConnectionManager<PgConnection>>,
    pub network_sender: Sender<Vec<u8>>,
}

impl State {
    pub fn new(
        redis: vm::redis::Client,
        rocksdb: Arc<rocksdb::DB>,
        db: Pool<ConnectionManager<PgConnection>>,
        network_sender: Sender<Vec<u8>>,
    ) -> Self {
        Self {
            websockets: Arc::new(Mutex::new(Vec::new())),
            redis,
            rocksdb,
            db,
            network_sender,
        }
    }

    pub async fn broadcast<M: Clone + Serialize>(&mut self, message: M) {
        self.network_sender
            .send(serde_cbor::to_vec(&message).unwrap())
            .await;
    }

    pub fn vm_state(&self) -> vm::State {
        match self {
            Self { redis, rocksdb, .. } => vm::State {
                redis: redis.get_connection().unwrap(),
                rocksdb: rocksdb.clone(),
                memory_changeset: HashMap::new(),
                storage_changeset: HashMap::new(),
            },
        }
    }
}
