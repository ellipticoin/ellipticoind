use crate::{aquire_db_write_lock, db, constants::DB, transaction::{new_update_transaction, new_start_bridge_transaction, run}};
use ellipticoin_contracts::{bridge::Update};
use std::task::Poll;

pub async fn start() {
    let ethereum_block_number = ellipticoin_peerchain_ethereum::get_current_block().await.unwrap();
    run(new_start_bridge_transaction(ethereum_block_number).await).await.unwrap();
}
pub async fn poll() {
    let ethereum_block_number = db::get_ethereum_block_number().await;
    match ellipticoin_peerchain_ethereum::poll(ethereum_block_number)
        .await
        .unwrap_or(Poll::Pending)
    {
        Poll::Ready(update) => {
            run(new_update_transaction(update).await).await.unwrap();
        }
        Poll::Pending => {}
    };
}
