use crate::models::{Block, Transaction};
use async_std::sync;
use network::Receiver;
use serde::{Deserialize, Serialize};
use vm::Commands;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Transaction(vm::Transaction),
    Block((Block, Vec<Transaction>)),
}

pub async fn handle_messages(
    mut redis: redis::Connection,
    mut network_receiver: Receiver,
    block_sender: sync::Sender<(Block, Vec<Transaction>)>,
) {
    loop {
        match &network_receiver.next().await {
            Ok(Message::Block((block, transactions))) => {
                block_sender
                    .send((block.clone(), transactions.clone()))
                    .await;
            }
            Ok(Message::Transaction(transaction)) => {
                redis
                    .rpush::<&str, Vec<u8>, ()>(
                        "transactions::pending",
                        serde_cbor::to_vec(&transaction).unwrap(),
                    )
                    .unwrap();
            }
            Err(_) => println!("Recieved an invalid message"),
        }
    }
}
