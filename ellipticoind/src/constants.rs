use crate::config::my_public_key;
use crate::{
    consensus::ExpectedBlock, models::Transaction, system_contracts::ellipticoin::Miner,
    transaction::TransactionRequest,
};
use async_std::sync::{channel, Mutex, Receiver, RwLock, Sender};
use broadcaster::BroadcastChannel;
use futures::channel::oneshot;
use std::{sync::Arc, time::Duration};

lazy_static! {
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(3);
    pub static ref BLOCK_SLASH_DELAY: Duration = Duration::from_secs(2);
    pub static ref TOKEN_CONTRACT: String = "Ellipticoin".to_string();
    pub static ref TRANSACTION_QUEUE_SIZE: usize = 1000;
    pub static ref TRANSACTION_QUEUE: (
        Sender<(TransactionRequest, oneshot::Sender<Transaction>)>,
        Receiver<(TransactionRequest, oneshot::Sender<Transaction>)>
    ) = channel(*TRANSACTION_QUEUE_SIZE);
    // TODO: Change the type of block channel
    pub static ref BLOCK_CHANNEL: (Sender<Miner>, Receiver<Miner>) = channel(1);
    pub static ref NEXT_BLOCK: Arc<RwLock<Option<ExpectedBlock>>> = Arc::new(RwLock::new(None));
    pub static ref MINERS: Arc<Mutex<Option<Vec<Miner>>>> = Arc::new(Mutex::new(None));
    pub static ref SYNCING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref BLOCK_BROADCASTER: BroadcastChannel<u32> = BroadcastChannel::new();
}

pub async fn set_miners(miners: Vec<Miner>) {
    *MINERS.lock().await = Some(miners.clone());
    BLOCK_CHANNEL
        .0
        .send(miners.first().clone().unwrap().clone())
        .await;
}

pub async fn get_my_miner() -> Miner {
    MINERS.me().await
}

impl MINERS {
    pub async fn current(&self) -> Miner {
        self.lock().await.as_ref().unwrap().first().unwrap().clone()
    }

    pub async fn second(&self) -> Miner {
        self.lock().await.as_ref().unwrap().get(1).unwrap().clone()
    }

    pub async fn count(&self) -> usize {
        self.lock().await.as_ref().unwrap().len()
    }

    // TODO: Cache this
    pub async fn me(&self) -> Miner {
        self.from_pub_key(my_public_key()).await
    }

    pub async fn from_pub_key(&self, key: [u8; 32]) -> Miner {
        self.lock()
            .await
            .as_ref()
            .unwrap()
            .iter()
            .filter(|m| m.address == key)
            .collect::<Vec<&Miner>>()[0]
            .clone()
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
