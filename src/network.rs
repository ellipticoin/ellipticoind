use crate::models::{Block, Transaction};

pub use futures::stream::StreamExt;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Transaction(crate::vm::Transaction),
    Block((Block, Vec<Transaction>)),
}
