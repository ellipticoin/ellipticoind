use crate::{
    config::{get_redis_connection, get_rocksdb, verification_key},
    models::Transaction,
    system_contracts::{
        api::ReadOnlyAPI,
        ellipticoin::{Miner, State},
    },
    transaction::TransactionRequest,
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
    pub static ref NEW_BLOCK_CHANNEL: (Sender<State>, Receiver<State>) = channel(1);
    pub static ref STATE: Arc<Mutex<State>> = {
        let mut read_only_api = ReadOnlyAPI::new(get_rocksdb(), get_redis_connection());

        let block_number =
            crate::system_contracts::ellipticoin::get_block_number(&mut read_only_api);
        let miners = crate::system_contracts::ellipticoin::get_miners(&mut read_only_api);
        Arc::new(Mutex::new(State {
            miners,
            block_number,
        }))
    };
    pub static ref SYNCING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref WEB_SOCKET_BROADCASTER: BroadcastChannel<(u32, String)> =
        BroadcastChannel::new();
}

impl STATE {
    pub async fn current_miner(&self) -> Miner {
        self.lock().await.miners.first().unwrap().clone()
    }

    pub async fn _second_miner(&self) -> Miner {
        self.lock().await.miners.get(1).unwrap().clone()
    }

    pub async fn current_onion_skin(&self) -> Option<[u8; 32]> {
        self.lock()
            .await
            .miners
            .iter()
            .find(|miner| miner.address == verification_key())
            .map(|miner| miner.hash_onion_skin)
    }

    pub async fn is_mining(&self) -> bool {
        self.lock()
            .await
            .miners
            .iter()
            .any(|miner| miner.address == verification_key())
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
