use crate::api;
use crate::models::{Block, Transaction};
use crate::transaction_processor;
use async_std::sync::{Receiver, Sender};
use network::StreamExt;
use serde::{Deserialize, Serialize};
use serde_cbor::from_slice;
use vm::Changeset;
use vm::Commands;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Transaction(vm::Transaction),
    Block((Block, Vec<Transaction>)),
}

pub async fn handle_messages(
    state: api::State,
    mut network_receiver: Receiver<Vec<u8>>,
    block_sender: Sender<(Changeset, Changeset, Block, Vec<Transaction>)>,
) {
    loop {
        let mut vm_state = state.vm_state();
        match from_slice(&network_receiver.next().await.unwrap()) {
            Ok(Message::Block((block, transactions))) => {
                transaction_processor::apply_block(
                    &mut vm_state,
                    block.clone(),
                    transactions.clone(),
                );
                block_sender
                    .send((
                        vm_state.memory_changeset,
                        vm_state.storage_changeset,
                        block,
                        transactions,
                    ))
                    .await;
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
