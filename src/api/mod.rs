use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

use crate::network;
use async_std::sync::Sender;
use futures::channel::mpsc::UnboundedSender;
use std::sync::{Arc, Mutex};
use tungstenite::protocol::Message;
pub use views::Block;
mod addresses;
pub mod app;
mod blocks;
mod memory;
mod storage;
mod transactions;
pub mod views;

pub mod websocket;

pub struct State {
    pub websockets: Arc<Mutex<Vec<UnboundedSender<Message>>>>,
    pub redis: crate::vm::r2d2_redis::r2d2::Pool<crate::vm::r2d2_redis::RedisConnectionManager>,
    pub rocksdb: Arc<rocksdb::DB>,
    pub db: Pool<ConnectionManager<PgConnection>>,
    pub sender_in: Sender<network::Message>,
}

impl State {
    pub fn new(
        redis: crate::vm::r2d2_redis::r2d2::Pool<crate::vm::r2d2_redis::RedisConnectionManager>,
        rocksdb: Arc<rocksdb::DB>,
        db: Pool<ConnectionManager<PgConnection>>,
        sender_in: Sender<network::Message>,
    ) -> Self {
        Self {
            websockets: Arc::new(Mutex::new(Vec::new())),
            redis,
            rocksdb,
            db,
            sender_in,
        }
    }
}
