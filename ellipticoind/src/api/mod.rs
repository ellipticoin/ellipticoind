use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use futures::channel::mpsc;
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
use crate::models;
pub mod websocket;

pub struct State {
    pub websockets: Arc<Mutex<Vec<UnboundedSender<Message>>>>,
    pub redis: vm::r2d2_redis::r2d2::Pool<vm::r2d2_redis::RedisConnectionManager>,
    pub rocksdb: Arc<rocksdb::DB>,
    pub db: Pool<ConnectionManager<PgConnection>>,
    pub block_sender_in: Sender<(models::Block, std::vec::Vec<models::Transaction>)>,
}

impl State {
    pub fn new(
        redis: vm::r2d2_redis::r2d2::Pool<vm::r2d2_redis::RedisConnectionManager>,
        rocksdb: Arc<rocksdb::DB>,
        db: Pool<ConnectionManager<PgConnection>>,
        block_sender_in: Sender<(models::Block, std::vec::Vec<models::Transaction>)>,
    ) -> Self {
        Self {
            websockets: Arc::new(Mutex::new(Vec::new())),
            redis,
            rocksdb,
            db,
            block_sender_in,
        }
    }
}
