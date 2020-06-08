use crate::api::views;
use crate::constants::{Namespace, TOKEN_CONTRACT};

use crate::network::Message;
use async_std::sync::Receiver;
use futures::stream::StreamExt;
use serde_cbor::Value;
use std::collections::BTreeMap;
use std::env;

pub async fn broadcast(mut block_receiver_out: Receiver<Message>, mut vm_state: crate::vm::State) {
    loop {
        match block_receiver_out.next().await.unwrap() {
            Message::Block((block, transactions)) => {
                let block: views::Block = (block, transactions).into();
                for peer in get_peers(&mut vm_state).await {
                    let uri = format!("http://{}/blocks", peer);
                    let _res = surf::post(uri)
                        .body_bytes(serde_cbor::to_vec(&block).unwrap())
                        .await
                        .unwrap();
                }
            }
            Message::Transaction(transaction) => {
                let transaction: crate::vm::Transaction = transaction.into();
                for peer in get_peers(&mut vm_state).await {
                    let uri = format!("http://{}/transactions", peer);
                    let _res = surf::post(uri)
                        .body_bytes(serde_cbor::to_vec(&transaction).unwrap())
                        .await
                        .unwrap();
                }
            }
        }
    }
}

pub async fn get_peers(vm_state: &mut crate::vm::State) -> Vec<String> {
    let miners: BTreeMap<Vec<Value>, (String, u64, Vec<Value>)> = serde_cbor::from_slice(
        &vm_state.get_storage(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]),
    )
    .unwrap();
    miners
        .iter()
        .map(|(_, (host, _, _))| host.clone())
        .filter(|host| host.to_string() != env::var("HOST").unwrap())
        .collect()
}
