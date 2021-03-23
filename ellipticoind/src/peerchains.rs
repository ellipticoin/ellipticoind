use crate::{
    db,
    transaction::{new_update_transaction, run},
};
use std::task::Poll;

// pub async fn start() {
//     let thing = ellipticoin_peerchain_ethereum::poll(0).await.unwrap();
//     println!("{:?}", thing);
//     let ethereum_block_number = ellipticoin_peerchain_ethereum::get_current_block()
//         .await
//         .unwrap();
//     run(new_start_bridge_transaction(ethereum_block_number).await)
//         .await
//         .unwrap();
// }
pub async fn poll() {
    let ethereum_block_number = db::get_ethereum_block_number().await;
    match ellipticoin_peerchain_ethereum::poll(ethereum_block_number)
        .await
        .unwrap_or(Poll::Pending)
    {
        Poll::Ready(update) => {
            let ethereum_block_number = db::get_ethereum_block_number().await;
            if ethereum_block_number + 1 == update.block_number {
                println!("Processed Ethereum Block #{}", ethereum_block_number);
            } else {
                println!(
                    "Processed Ethereum Block #{}-#{}",
                    ethereum_block_number + 1,
                    update.block_number
                );
            }
            run(new_update_transaction(update).await).await.unwrap();
        }
        Poll::Pending => {}
    };
}
