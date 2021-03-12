use crate::config::verification_key;
use crate::constants::DB;

use crate::{
    constants::{NETWORK_ID, TRANSACTION_QUEUE, TRANSACTIONS_FILE},
    crypto::{recover, sign, sign_eth},
};
use anyhow::Result;
use ellipticoin_contracts::{Action, Transaction};
use ellipticoin_contracts::{Bridge, System};
use ellipticoin_peerchain_ethereum::{
    constants::{BRIDGE_ADDRESS, REDEEM_TIMEOUT},
    SignedTransaction,
};
use ellipticoin_types::db::{Backend, Db};
use ellipticoin_types::traits::Run;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum SignedTransaction2 {
    Ethereum(ellipticoin_peerchain_ethereum::SignedTransaction),
    System(SignedSystemTransaction),
}

impl SignedTransaction2 {
    fn is_redeem_request(&self) -> bool {
        if let SignedTransaction2::Ethereum(transaction) = self {
            matches!(transaction.0.action, Action::CreateRedeemRequest(_, _))
        } else {
            false
        }
    }

    pub fn is_seal(&self) -> bool {
        if let SignedTransaction2::System(transaction) = self {
            matches!(transaction.0.action, Action::Seal(_))
        } else {
            false
        }
    }
}
impl Run for SignedTransaction2 {
    fn sender(&self) -> Result<[u8; 20]> {
        match self {
            SignedTransaction2::Ethereum(transaction) => transaction.sender(),
            SignedTransaction2::System(transaction) => transaction.sender(),
        }
    }

    fn run<B: Backend>(&self, db: &mut Db<B>) -> Result<()> {
        match self {
            SignedTransaction2::Ethereum(transaction) => transaction.run(db),
            SignedTransaction2::System(transaction) => transaction.run(db),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedSystemTransaction(pub Transaction, Vec<u8>);

impl Run for SignedSystemTransaction {
    fn sender(&self) -> Result<[u8; 20]> {
        recover(&serde_cbor::to_vec(&self.0)?, &self.1)
    }

    fn run<B: Backend>(&self, db: &mut Db<B>) -> Result<()> {
        self.0.action.run(db, self.sender()?)
    }
}

pub trait IsRedeemRequest {
    fn is_redeem_request(&self) -> bool;
}

impl IsRedeemRequest for SignedTransaction {
    fn is_redeem_request(&self) -> bool {
        matches!(self.0.action, Action::CreateRedeemRequest(_, _))
    }
}

impl IsRedeemRequest for SignedSystemTransaction {
    fn is_redeem_request(&self) -> bool {
        false
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

    pub async fn apply<B: Backend>(&self, db: &mut Db<B>) -> Result<()> {
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

// pub async fn run<R: Run + IsRedeemRequest + Serialize>(transaction: R) -> Result<()> {
pub async fn run<B: Backend>(transaction: SignedTransaction2, db: &mut Db<B>) -> Result<()> {
    // let backend = DB.get().unwrap().write().await;
    // let store_lock = crate::db::StoreLock { guard: backend };
    // let mut db = ellipticoin_types::Db {
    //     backend: store_lock,
    //     transaction_state: Default::default(),
    // };
    println!("1");
    let result = transaction.run(db);
    println!("2");
    if transaction.is_redeem_request() && result.is_ok() {
        sign_last_redeem_request(db).await.unwrap();
    }
    if result.is_ok() {
        db.commit();
    } else {
        db.revert();
    }
    println!("3");
    serde_cbor::to_writer(&*TRANSACTIONS_FILE, &transaction).unwrap();
    println!("4");

    result
}

pub async fn apply(transaction: &SignedTransaction2) -> Result<()> {
    let backend = DB.get().unwrap().write().await;
    let store_lock = crate::db::StoreLock { guard: backend };
    let mut db = ellipticoin_types::Db {
        backend: store_lock,
        transaction_state: Default::default(),
    };
    let result = transaction.run(&mut db);
    if transaction.is_redeem_request() && result.is_ok() {
        sign_last_redeem_request(&mut db).await.unwrap();
    }
    if result.is_ok() {
        db.commit();
    } else {
        db.revert();
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
    redeem_transaction.run(db)
}
