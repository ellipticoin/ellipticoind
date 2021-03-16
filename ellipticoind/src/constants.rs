use crate::{db, db::sled_backend::SledBackend, transaction::SignedTransaction};
use anyhow::Result;
use async_std::{
    channel::{self, Receiver, Sender},
    sync::{Mutex, RwLock},
};
use broadcaster::BroadcastChannel;
use futures::channel::oneshot;
use once_cell::sync::OnceCell;
use std::{
    fs::{File, OpenOptions},
    sync::Arc,
    time::Duration,
};

pub const NETWORK_ID: u64 = 0;
pub static DB: OnceCell<RwLock<SledBackend>> = OnceCell::new();

lazy_static! {
    pub static ref TRANSACTIONS_FILE: File = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("var/transactions.cbor")
        .unwrap();
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(4);
    pub static ref TRANSACTION_QUEUE_SIZE: usize = 1000;
    pub static ref TRANSACTION_QUEUE: (
        Sender<(SignedTransaction, oneshot::Sender<Result<()>>)>,
        Receiver<(SignedTransaction, oneshot::Sender<Result<()>>)>
    ) = channel::bounded(*TRANSACTION_QUEUE_SIZE);
    pub static ref SYNCING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref WEB_SOCKET_BROADCASTER: BroadcastChannel<(u32, String)> =
        BroadcastChannel::new();
    pub static ref SLED_DB: sled::Db = sled::open("var/db").unwrap();
}

impl TRANSACTION_QUEUE {
    pub async fn push(&self, transaction: SignedTransaction) -> oneshot::Receiver<Result<()>> {
        let (sender, receiver) = oneshot::channel();
        self.0.send((transaction, sender)).await.unwrap();
        receiver
    }

    pub async fn process_next_transaction(&self) {
        let (transaction, sender) = self.1.recv().await.unwrap();
        sender
            .send(crate::transaction::run(transaction).await)
            .unwrap();
    }
}

impl WEB_SOCKET_BROADCASTER {
    pub async fn broadcast(&self) {
        let current_miner = db::get_current_miner().await.unwrap();
        let block_number = db::get_block_number().await;
        self.send(&(block_number as u32, current_miner.host))
            .await
            .unwrap();
    }
}
