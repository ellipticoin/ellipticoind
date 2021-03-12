use crate::constants::DB;
use ellipticoin_types::db::{Db,Backend};
use crate::{
    constants::{BLOCK_TIME, TRANSACTION_QUEUE, WEB_SOCKET_BROADCASTER},
    hash_onion,
    helpers::run_for,
    aquire_db_read_lock,aquire_db_write_lock, peerchains,
    transaction::{self, SignedSystemTransaction, SignedTransaction2},
};
use ellipticoin_contracts::{Action, Ellipticoin, System};

pub async fn run() {
    loop {
        mine_block().await
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
async fn mine_block() {
    let block_number = {
        let mut db = aquire_db_read_lock!();
        System::get_block_number(&mut db)
    };
    println!("Won block #{}", block_number);
    run_for(*BLOCK_TIME, async {
        loop {
            let (transaction, sender) = TRANSACTION_QUEUE.1.recv().await.unwrap();
            sender
                .send(crate::transaction::run(SignedTransaction2::Ethereum(transaction)).await)
                .unwrap();
        }
    })
    .await;
    peerchains::poll().await;
    println!("1");
    // let mut db = aquire_db_read_lock!();
    println!("2");
    run_seal().await;
    println!("3");
    let mut db = aquire_db_write_lock!();
    let current_miner = Ellipticoin::get_miners(&mut db)
            .first()
            .unwrap()
            .host
            .clone();
    println!("4");
    WEB_SOCKET_BROADCASTER
        .broadcast(System::get_block_number(&mut db), current_miner)
        .await;
    db.flush();
}

async fn run_seal() {
    let seal_transaction = {
        let mut db = aquire_db_read_lock!();
        SignedSystemTransaction::new(&mut db, Action::Seal(hash_onion::peel().await))
    };
    transaction::run(SignedTransaction2::System(seal_transaction))
        .await
        .unwrap();
}
