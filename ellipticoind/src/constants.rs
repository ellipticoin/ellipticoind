use crate::{
    models::Transaction, system_contracts::ellipticoin::Miner, transaction::TransactionRequest,
};
use async_std::sync::{channel, Mutex, Receiver, Sender};
use broadcaster::BroadcastChannel;
use futures::channel::oneshot;
use std::{sync::Arc, time::Duration};

lazy_static! {
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(3);
    pub static ref TOKEN_CONTRACT: String = "Ellipticoin".to_string();
    pub static ref TRANSACTION_QUEUE_SIZE: usize = 1000;
    pub static ref TRANSACTION_QUEUE: (
        Sender<(TransactionRequest, oneshot::Sender<Transaction>)>,
        Receiver<(TransactionRequest, oneshot::Sender<Transaction>)>
    ) = channel(*TRANSACTION_QUEUE_SIZE);
    pub static ref CURRENT_MINER_CHANNEL: (Sender<Miner>, Receiver<Miner>) = channel(1);
    pub static ref MINERS: Arc<Mutex<Option<Vec<Miner>>>> = Arc::new(Mutex::new(None));
    pub static ref SYNCING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref WEB_SOCKET_BROADCASTER: BroadcastChannel<(u32, String)> =
        BroadcastChannel::new();
}

pub async fn set_miners(miners: Vec<Miner>) {
    *MINERS.lock().await = Some(miners.clone());
    CURRENT_MINER_CHANNEL
        .0
        .send(miners.first().clone().unwrap().clone())
        .await;
}
impl MINERS {
    pub async fn current(&self) -> Miner {
        self.lock().await.as_ref().unwrap().first().unwrap().clone()
    }

    pub async fn _second(&self) -> Miner {
        self.lock().await.as_ref().unwrap().get(1).unwrap().clone()
    }
}

impl TRANSACTION_QUEUE {
    pub async fn push(
        &self,
        transaction_request: TransactionRequest,
    ) -> oneshot::Receiver<Transaction> {
        let (sender, receiver) = oneshot::channel();
        self.0.send((transaction_request, sender)).await;
        receiver
    }
}

impl WEB_SOCKET_BROADCASTER {
    pub async fn broadcast(&self, block_number: u32, current_miner_host: String) {
        self.send(&(block_number, current_miner_host))
            .await
            .unwrap();
    }
}
