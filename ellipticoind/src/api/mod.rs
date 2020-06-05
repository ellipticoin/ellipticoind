use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use futures::channel::mpsc;
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
    pub redis: vm::r2d2_redis::r2d2::Pool<vm::r2d2_redis::RedisConnectionManager>,
    pub rocksdb: Arc<rocksdb::DB>,
    pub db: Pool<ConnectionManager<PgConnection>>,
}

impl State {
    pub fn new(
        redis: vm::r2d2_redis::r2d2::Pool<vm::r2d2_redis::RedisConnectionManager>,
        rocksdb: Arc<rocksdb::DB>,
        db: Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        Self {
            websockets: Arc::new(Mutex::new(Vec::new())),
            redis,
            rocksdb,
            db,
        }
    }
}
