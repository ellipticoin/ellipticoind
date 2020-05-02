use crate::models::{Block, Transaction};
use async_std::sync;
use serde::{Deserialize};
use network::serde::Serialize;
use vm::Commands;
use futures::channel::mpsc::Receiver;
pub use futures::{
    stream::StreamExt,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Transaction(vm::Transaction),
    Block((Block, Vec<Transaction>)),
}

pub async fn handle_messages(
    mut redis: redis::Connection,
    mut network_receiver: Receiver<Message>,
    block_sender: sync::Sender<(Block, Vec<Transaction>)>,
) {
    loop {
        match &network_receiver.next().await {
            Some(Message::Block((block, transactions))) => {
                block_sender
                    .send((block.clone(), transactions.clone()))
                    .await;
            }
            Some(Message::Transaction(transaction)) => {
                redis
                    .rpush::<&str, Vec<u8>, ()>(
                        "transactions::pending",
                        serde_cbor::to_vec(&transaction).unwrap(),
                    )
                    .unwrap();
            }
            None => println!("Recieved an invalid message"),
        }
    }
}
