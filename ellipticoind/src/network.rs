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
                println!("received tx");
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
