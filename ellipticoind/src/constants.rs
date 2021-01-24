use anyhow::Result;
use async_std::{
    channel::{self, Receiver, Sender},
    sync::Mutex,
};
use broadcaster::BroadcastChannel;
use ellipticoin_peerchain_ethereum::SignedTransaction;
use futures::channel::oneshot;
use std::{sync::Arc, time::Duration};

pub const NETWORK_ID: u64 = 0;

lazy_static! {
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(4);
    pub static ref TRANSACTION_QUEUE_SIZE: usize = 1000;
    pub static ref TRANSACTION_QUEUE: (
        Sender<(SignedTransaction, oneshot::Sender<Result<()>>)>,
        Receiver<(SignedTransaction, oneshot::Sender<Result<()>>)>
    ) = channel::bounded(*TRANSACTION_QUEUE_SIZE);
    pub static ref SYNCING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref WEB_SOCKET_BROADCASTER: BroadcastChannel<(u32, String)> =
        BroadcastChannel::new();
}

impl TRANSACTION_QUEUE {
    pub async fn push(&self, transaction: SignedTransaction) -> oneshot::Receiver<Result<()>> {
        let (sender, receiver) = oneshot::channel();
        self.0.send((transaction, sender)).await.unwrap();
        receiver
    }
}

impl WEB_SOCKET_BROADCASTER {
    pub async fn broadcast(&self, block_number: u64, current_miner_host: String) {
        self.send(&(block_number as u32, current_miner_host))
            .await
            .unwrap();
    }
}
