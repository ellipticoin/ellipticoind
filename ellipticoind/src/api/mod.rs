use futures::{FutureExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};
use serde::Serialize;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

#[derive(Clone)]
pub struct API {
    pub users: Arc<Mutex<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>,
    pub redis: vm::redis::Client,
    pub db: Pool<ConnectionManager<PgConnection>>,
}
pub mod blocks;
pub mod memory;
pub mod routes;
pub mod transactions;
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Serialize)]
pub struct Block {
    #[serde(with = "serde_bytes")]
    pub hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub number: i64,
    #[serde(with = "serde_bytes")]
    pub winner: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub memory_changeset_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub storage_changeset_hash: Vec<u8>,
    pub proof_of_work_value: i64,
    pub transactions: Vec<Transaction>,
}

#[derive(Serialize)]
pub struct Transaction {
    pub arguments: Vec<serde_cbor::Value>,
    #[serde(with = "serde_bytes")]
    pub block_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub contract_address: Vec<u8>,
    pub function: String,
    pub gas_limit: u64,
    pub nonce: u64,
    return_code: u64,
    return_value: serde_cbor::Value,
    #[serde(with = "serde_bytes")]
    pub sender: Vec<u8>,
}

impl From<(&crate::models::Block, &Vec<crate::models::Transaction>)> for Block {
    fn from(block: (&crate::models::Block, &Vec<crate::models::Transaction>)) -> Self {
        Self {
            hash: block.0.hash.clone(),
            parent_hash: block.0.parent_hash.clone(),
            number: block.0.number,
            winner: block.0.winner.clone(),
            memory_changeset_hash: block.0.memory_changeset_hash.clone(),
            storage_changeset_hash: block.0.storage_changeset_hash.clone(),
            proof_of_work_value: block.0.proof_of_work_value.clone(),
            transactions: block
                .1
                .into_iter()
                .map(Transaction::from)
                .collect::<Vec<Transaction>>(),
        }
    }
}

impl From<&crate::models::Transaction> for Transaction {
    fn from(transaction: &crate::models::Transaction) -> Self {
        Self {
            contract_address: transaction.contract_address.clone(),
            block_hash: transaction.block_hash.clone(),
            sender: transaction.sender.clone(),
            nonce: transaction.nonce as u64,
            gas_limit: transaction.gas_limit as u64,
            function: transaction.function.clone(),
            arguments: serde_cbor::from_slice(&transaction.arguments).unwrap(),
            return_value: serde_cbor::from_slice(&transaction.return_value).unwrap(),
            return_code: transaction.return_code as u64,

        }
    }
}

impl API {
    pub fn new(redis: vm::redis::Client, db: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            redis,
            db,
        }
    }

    pub async fn serve(self, socket: SocketAddr) {
        warp::serve(routes::routes(self)).run(socket).await;
    }

    pub async fn broadcast_block(&mut self, block: Block) {
        self.broadcast(serde_cbor::to_vec(&block).unwrap()).await;
    }
    pub async fn broadcast<V: Clone + Into<Vec<u8>>>(&mut self, message: V) {
        for (&_uid, tx) in self.users.lock().unwrap().iter_mut() {
            tx.try_send(Ok(Message::binary(message.clone()))).unwrap();
        }
    }

    async fn user_connected(&mut self, ws: WebSocket) {
        let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
        let (user_ws_tx, user_ws_rx) = ws.split();
        let (tx, rx) = mpsc::unbounded_channel();
        warp::spawn(rx.forward(user_ws_tx).map(|_result| {}));
        self.users.lock().unwrap().insert(my_id, tx);
        let mut api = self.clone();
        user_ws_rx
            .for_each(async move |_msg| ())
            .then(async move |result| {
                api.user_disconnected(my_id);
                Ok::<(), ()>(result)
            })
            .await
            .unwrap();
    }

    fn user_disconnected(&mut self, my_id: usize) {
        self.users.lock().unwrap().remove(&my_id);
    }
}
