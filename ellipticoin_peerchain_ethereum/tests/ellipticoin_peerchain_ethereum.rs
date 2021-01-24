use async_std::{
    sync::{Arc, Mutex},
    task,
};
use ellipticoin_peerchain_ethereum::Update;
use lazy_static::lazy_static;
use std::{task::Poll, time::Duration};

lazy_static! {
    pub static ref LATEST_BLOCK: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
}

#[test]
fn test_add() {
    async_std::task::block_on(async {
        *LATEST_BLOCK.lock().await = ellipticoin_peerchain_ethereum::get_current_block()
            .await
            .unwrap();
        loop {
            let latest_block = LATEST_BLOCK.lock().await.clone();
            match ellipticoin_peerchain_ethereum::poll(latest_block)
                .await
                .unwrap()
            {
                Poll::Ready(Update {
                    block_number,
                    mints,
                    redeems,
                }) => {
                    println!("ready {} {:?} {:?}", block_number, mints, redeems);
                    *LATEST_BLOCK.lock().await = block_number;
                }
                Poll::Pending => task::sleep(Duration::from_secs(3)).await,
            }
        }
    })
}
