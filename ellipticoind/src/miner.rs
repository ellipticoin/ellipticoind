use crate::{
    constants::{BLOCK_TIME, TRANSACTION_QUEUE, WEB_SOCKET_BROADCASTER},
    db::MemoryDB,
    hash_onion,
    helpers::run_for,
    peerchains,
    state::IN_MEMORY_STATE,
    transaction::SignedSystemTransaction,
};
use ellipticoin_contracts::{Action, Ellipticoin, System};

pub async fn run() {
    loop {
        mine_block(1).await
    }
    // loop {
    //     match timeout(
    //         *BLOCK_TIME + Duration::from_secs(2),
    //         NEW_BLOCK_CHANNEL.1.recv().map(Result::unwrap),
    //     )
    //     .await
    //     {
    //         Ok(state) => mine_if_winner(state).await,
    //         Err(TimeoutError { .. }) => wait_for_peer().await,
    //     }
    // }
}

async fn _wait_for_peer() {
    println!("waitng for peers");
    // let current_miner = current_miner().await;
    // println!(
    //     "Waiting for peer: {} ({})",
    //     current_miner.host,
    //     base64::encode(&current_miner.address)
    // );
    // let state = NEW_BLOCK_CHANNEL.1.recv().map(Result::unwrap).await;
    // mine_if_winner(state).await
}

// async fn mine_if_winner(state: State) {
//     if state
//         .miners
//         .first()
//         .map(|miner| miner.address == verification_key())
//         .unwrap_or(false)
//     {
//         mine_block(state.block_number).await
//     } else {
//         sleep(*BLOCK_TIME).await;
//     }
// }
async fn mine_block(_block_number: u32) {
    let block_number = {
        let mut state = IN_MEMORY_STATE.lock().await;
        let mut db = MemoryDB::new(&mut state);
        System::get_block_number(&mut db)
    };
    println!("Won block #{}", block_number);
    run_for(*BLOCK_TIME, async {
        loop {
            let (transaction, sender) = TRANSACTION_QUEUE.1.recv().await.unwrap();
            sender
                .send(crate::transaction::run(transaction).await)
                .unwrap();
        }
    })
    .await;
    let mut state = IN_MEMORY_STATE.lock().await;
    let mut db = MemoryDB::new(&mut state);
    peerchains::poll(&mut db).await;
    let seal_transaction =
        SignedSystemTransaction::new(&mut db, Action::Seal(hash_onion::peel().await));
    seal_transaction.run(&mut db).await.unwrap();
    let current_miner = Ellipticoin::get_miners(&mut db)
        .first()
        .unwrap()
        .host
        .clone();
    WEB_SOCKET_BROADCASTER
        .broadcast(block_number + 1, current_miner)
        .await;
}
