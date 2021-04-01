use async_std::{
    sync::{Arc, Mutex},
    task,
};
use ellipticoin_contracts::bridge::Update;
use lazy_static::lazy_static;
use std::{task::Poll, time::Duration};

lazy_static! {
    pub static ref LATEST_BLOCK: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
}

#[test]
fn test_add() {
    async_std::task::block_on(async {
        println!("hi");
        *LATEST_BLOCK.lock().await = ellipticoin_peerchain_ethereum::get_current_block()
            .await
            .unwrap();
        loop {
            let latest_block = LATEST_BLOCK.lock().await.clone();
            println!("{}", latest_block);
            match ellipticoin_peerchain_ethereum::poll(latest_block)
                .await
                .unwrap()
            {
                Poll::Ready(Update {
                    block_number,
                    base_token_interest_rate,
                    base_token_exchange_rate,
                    mints,
                    redeems,
                }) => {
                    println!(
                        "ready {} {} {} {:?} {:?}",
                        block_number,
                        base_token_exchange_rate,
                        base_token_interest_rate,
                        mints,
                        redeems
                    );
                    *LATEST_BLOCK.lock().await = block_number;
                }
                Poll::Pending => task::sleep(Duration::from_secs(3)).await,
            }
        }
    })
}
