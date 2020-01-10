use crate::api;
use crate::models::{Block, Transaction};
use async_std::sync::{Receiver, Sender};
use network::StreamExt;
use serde::{Deserialize, Serialize};
use serde_cbor::from_slice;
use vm::Commands;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Transaction(vm::Transaction),
    Block((Block, Vec<Transaction>)),
}

pub async fn handle_messages(
    state: api::State,
    mut network_receiver: Receiver<Vec<u8>>,
    block_sender: Sender<(Block, Vec<Transaction>)>,
) {
    loop {
        match from_slice(&network_receiver.next().await.unwrap()) {
            Ok(Message::Block((block, transactions))) => {
                block_sender.send((block, transactions)).await;
            }
            Ok(Message::Transaction(transaction)) => {
                let mut redis = state.redis.get_connection().unwrap();
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
