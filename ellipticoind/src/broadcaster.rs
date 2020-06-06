use crate::constants::{Namespace, TOKEN_CONTRACT};
use crate::models::{Block, Transaction};
use crate::api::views;
use async_std::sync::Receiver;
use futures::stream::StreamExt;
use serde_cbor::Value;
use std::collections::BTreeMap;
use std::env;

pub async fn broadcast(
    mut block_receiver_out: Receiver<(Block, Vec<Transaction>)>,
    mut vm_state: vm::State,
) {
    loop {
        let block: views::Block = block_receiver_out.next().await.unwrap().into();
        for peer in get_peers(&mut vm_state).await {
            let uri = format!("http://{}/blocks", peer);
            let _res = surf::post(uri)
                .body_bytes(serde_cbor::to_vec(&block).unwrap())
                .await
                .unwrap();
        }
    }
}

pub async fn get_peers(vm_state: &mut vm::State) -> Vec<String> {
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
