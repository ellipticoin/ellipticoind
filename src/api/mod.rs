use crate::network;
use async_std::sync::Sender;
use futures::channel::mpsc::UnboundedSender;
use std::sync::{Arc, Mutex};
use tungstenite::protocol::Message;
pub use views::Block;
mod addresses;
pub mod app;
mod blocks;
mod helpers;
mod memory;
pub mod state;
mod storage;
mod transactions;
pub mod views;
pub mod websocket;
pub struct ApiState {
    pub websockets: Arc<Mutex<Vec<UnboundedSender<Message>>>>,
    pub miner_sender: Sender<network::Message>,
}
impl ApiState {
    pub fn new(miner_sender: Sender<network::Message>) -> Self {
        Self {
            websockets: Arc::new(Mutex::new(Vec::new())),
            miner_sender,
        }
    }
}
