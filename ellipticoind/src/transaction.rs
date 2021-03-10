use crate::config::verification_key;
use crate::constants::DB;
use crate::{
    constants::{NETWORK_ID, TRANSACTION_QUEUE},
    crypto::{recover, sign, sign_eth},
};
use anyhow::Result;
use ellipticoin_contracts::{Action, Run, Transaction};
use ellipticoin_contracts::{Bridge, System};
use ellipticoin_peerchain_ethereum::{
    constants::{BRIDGE_ADDRESS, REDEEM_TIMEOUT},
    Signed, SignedTransaction,
};
use ellipticoin_types::db::{Backend, Db};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedSystemTransaction(Transaction<Action>, Vec<u8>);

impl Signed for SignedSystemTransaction {
    fn sender(&self) -> Result<[u8; 20]> {
        recover(&serde_cbor::to_vec(&self.0)?, &self.1)
    }
}

impl SignedSystemTransaction {
    pub fn new<B: Backend>(db: &mut Db<B>, action: Action) -> SignedSystemTransaction {
        let transaction = Transaction {
            network_id: NETWORK_ID,
            transaction_number: System::get_next_transaction_number(db, verification_key()),
            action,
        };
        let signature = sign(&transaction);
        SignedSystemTransaction(transaction, signature.to_vec())
    }

    pub async fn run<B: Backend>(&self, db: &mut Db<B>) -> Result<()> {
        let result = self.0.action.run(db, self.sender()?);
        if result.is_ok() {
            db.commit();
        } else {
            db.revert();
        }
        result
    }
}

pub async fn dispatch(signed_transaction: SignedTransaction) -> Result<()> {
    let receiver = TRANSACTION_QUEUE.push(signed_transaction).await;
    receiver.await.unwrap()
}

pub async fn run(signed_transaction: SignedTransaction) -> Result<()> {
    let mut db = DB.get().unwrap().write().await;
    let mut backend = DB.get().unwrap().write().await;
    let store_lock = crate::db::StoreLock{guard: backend};
    let mut db = ellipticoin_types::Db {
backend: store_lock,
             transaction_state: Default::default(),
    };
    let result = signed_transaction.run(&mut db).await;
    if matches!(
        signed_transaction.0.action,
        Action::CreateRedeemRequest(_, _)
    ) && result.is_ok()
    {
        sign_last_redeem_request(&mut db).await.unwrap();
    }
    result
}

pub async fn sign_last_redeem_request<B: Backend>(db: &mut Db<B>) -> Result<()> {
    let pending_redeem_request = Bridge::get_pending_redeem_requests(db)
        .last()
        .unwrap()
        .clone();
    let ethereum_block_number = Bridge::get_ethereum_block_number(db);
    let experation_block_number = ethereum_block_number + REDEEM_TIMEOUT;
    let signature = sign_eth(&(
        serde_eth::Address(pending_redeem_request.sender),
        pending_redeem_request.amount,
        serde_eth::Address(pending_redeem_request.token),
        experation_block_number,
        pending_redeem_request.id,
        serde_eth::Address(BRIDGE_ADDRESS),
    ));

    let redeem_transaction = SignedSystemTransaction::new(
        db,
        Action::SignRedeemRequest(
            pending_redeem_request.id,
            experation_block_number,
            signature.to_vec(),
        ),
    );
    redeem_transaction.run(db).await
}
