use crate::models::{Block, Transaction};
use async_std::sync;
use futures::channel::mpsc::Receiver;
pub use futures::stream::StreamExt;
use network::serde::Serialize;
use serde::Deserialize;
use vm::Commands;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Transaction(vm::Transaction),
    Block((Block, Vec<Transaction>)),
}
