use crate::{
    aquire_db_read_lock, aquire_db_write_lock,
    config::{verification_key, HOST},
    constants::{DB, NETWORK_ID, TRANSACTIONS_FILE, TRANSACTION_QUEUE},
    crypto::{sign, sign_eth},
    hash_onion,
};
use anyhow::Result;
use ellipticoin_contracts::bridge::Update;
use ellipticoin_contracts::{Action, Bridge, System, Transaction};
use ellipticoin_peerchain_ethereum::constants::{BRIDGE_ADDRESS, REDEEM_TIMEOUT};
use ellipticoin_peerchain_ethereum::ecrecover;
use ellipticoin_types::Address;
use ellipticoin_types::{
    db::{Backend, Db},
    traits::Run,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum SignedTransaction {
    Ethereum(ellipticoin_peerchain_ethereum::SignedTransaction),
    System(SignedSystemTransaction),
}

impl SignedTransaction {
    fn is_redeem_request(&self) -> bool {
        if let SignedTransaction::Ethereum(transaction) = self {
            matches!(transaction.0.action, Action::CreateRedeemRequest(_, _))
        } else {
            false
        }
    }

    pub fn is_seal(&self) -> bool {
        if let SignedTransaction::System(transaction) = self {
            matches!(transaction.0.action, Action::Seal(_))
        } else {
            false
        }
    }
}
impl Run for SignedTransaction {
    fn sender(&self) -> Result<Address> {
        match self {
            SignedTransaction::Ethereum(transaction) => transaction.sender(),
            SignedTransaction::System(transaction) => transaction.sender(),
        }
    }

    fn run<B: Backend>(&self, db: &mut Db<B>) -> Result<()> {
        match self {
            SignedTransaction::Ethereum(transaction) => transaction.run(db),
            SignedTransaction::System(transaction) => transaction.run(db),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedSystemTransaction(pub Transaction, Vec<u8>);

impl Run for SignedSystemTransaction {
    fn sender(&self) -> Result<Address> {
        ecrecover(serde_cbor::to_vec(&self.0)?, &self.1)
    }

    fn run<B: Backend>(&self, db: &mut Db<B>) -> Result<()> {
        self.0.action.run(db, self.sender()?)
    }
}

pub trait IsRedeemRequest {
    fn is_redeem_request(&self) -> bool;
}

impl IsRedeemRequest for ellipticoin_peerchain_ethereum::SignedTransaction {
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

pub async fn run(transaction: SignedTransaction) -> Result<()> {
    let mut db = aquire_db_write_lock!();
    let result = transaction.run(&mut db);
    if transaction.is_redeem_request() && result.is_ok() {
        sign_last_redeem_request(&mut db).await.unwrap();
    }
    if result.is_ok() {
        db.commit();
    } else {
        db.revert();
    }
    // println!("writing to file: {:?}", transaction);
    serde_cbor::to_writer(&*TRANSACTIONS_FILE, &transaction).unwrap();

    result
}

pub async fn apply(transaction: &SignedTransaction) -> Result<()> {
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
        serde_eth::Address(pending_redeem_request.sender.0),
        pending_redeem_request.amount,
        serde_eth::Address(pending_redeem_request.token.0),
        experation_block_number,
        pending_redeem_request.id,
        serde_eth::Address(BRIDGE_ADDRESS.0),
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

pub async fn new_start_mining_transaction() -> SignedTransaction {
    let mut db = aquire_db_read_lock!();
    SignedTransaction::System(SignedSystemTransaction::new(
        &mut db,
        Action::StartMining(
            HOST.to_string(),
            hash_onion::peel().await,
            hash_onion::layers_left().await as u64,
        ),
    ))
}

pub async fn new_seal_transaction() -> SignedTransaction {
    let mut db = aquire_db_read_lock!();
    let seal_transaction =
        SignedSystemTransaction::new(&mut db, Action::Seal(hash_onion::peel().await));
    SignedTransaction::System(seal_transaction)
}

pub async fn new_start_bridge_transaction(ethereum_block_number: u64) -> SignedTransaction {
    let mut db = aquire_db_read_lock!();
    let start_bridge_transaction =
        SignedSystemTransaction::new(&mut db, Action::StartBridge(ethereum_block_number));
    SignedTransaction::System(start_bridge_transaction)
}

pub async fn new_update_transaction(update: Update) -> SignedTransaction {
    let mut db = aquire_db_read_lock!();
    let update_transaction = SignedSystemTransaction::new(&mut db, Action::Update(update));
    SignedTransaction::System(update_transaction)
}
