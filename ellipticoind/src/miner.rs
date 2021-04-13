use crate::{
    constants::{BLOCK_TIME, TRANSACTION_QUEUE, WEB_SOCKET_BROADCASTER},
    db,
    helpers::run_for,
    peerchains,
    transaction::{self, new_seal_transaction},
};

pub async fn run() {
    loop {
        mine_block().await
    }
}

async fn mine_block() {
    println!("Won block #{}", db::get_block_number().await);
    run_for(*BLOCK_TIME, async {
        loop {
            TRANSACTION_QUEUE.process_next_transaction().await
        }
    })
    .await;
    peerchains::poll().await;
    transaction::run(new_seal_transaction().await)
        .await
        .unwrap();
    WEB_SOCKET_BROADCASTER.broadcast().await;
    db::flush().await;
}
