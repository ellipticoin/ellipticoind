use crate::models::{Block, Transaction};

pub use futures::stream::StreamExt;
use network::serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Transaction(vm::Transaction),
    Block((Block, Vec<Transaction>)),
}
