use ellipticoin_contracts::Bridge;
use ellipticoin_types::db::{Db, Backend};
use ellipticoin_peerchain_ethereum::{Mint, Redeem, Update};
use std::task::Poll;

pub async fn start<'a, B: Backend>(db: &mut Db<B>) {
    let ethereum_block_number = ellipticoin_peerchain_ethereum::get_current_block()
        .await
        .unwrap();
    Bridge::set_ethereum_block_number(db, ethereum_block_number);
    db.commit();
}
pub async fn poll<'a, B: Backend>(db: &mut Db<B>) {
    let ethereum_block_number = Bridge::get_ethereum_block_number(db);
    match ellipticoin_peerchain_ethereum::poll(ethereum_block_number)
        .await
        .unwrap_or(Poll::Pending)
    {
        Poll::Ready(Update {
            block_number,
            mints,
            redeems,
        }) => {
            Bridge::set_ethereum_block_number(db, block_number);
            let pending_redeem_requests = Bridge::get_pending_redeem_requests(db);
            for pending_redeem_request in pending_redeem_requests.iter() {
                if block_number > pending_redeem_request.expiration_block_number.unwrap() {
                    println!("Redeem {} timed out", pending_redeem_request.id);
                    Bridge::cancel_redeem_request(db, pending_redeem_request.id).unwrap();
                }
            }
            for Mint(amount, token, address) in mints.iter() {
                println!(
                    "minted {} {} to {}",
                    amount,
                    hex::encode(token),
                    hex::encode(address)
                );
                Bridge::mint(db, *amount, *token, *address).unwrap();
            }
            for Redeem(redeem_id) in redeems.iter() {
                println!("redeemed id: {}", redeem_id);
                Bridge::redeem(db, *redeem_id).unwrap();
            }
            db.commit();
            if ethereum_block_number + 1 == block_number {
                println!("Processed Ethereum Block #{}", block_number);
            } else {
                println!(
                    "Processed Ethereum Block #{}-#{}",
                    ethereum_block_number + 1,
                    block_number
                );
            }
        }
        Poll::Pending => (),
    }
}
