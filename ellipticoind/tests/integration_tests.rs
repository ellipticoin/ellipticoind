extern crate ellipticoind;

#[macro_use]
extern crate lazy_static;

use std::include_bytes;
use vm::Transaction;
mod helpers;
use core::time::Duration;
use helpers::{get_balance, set_balance, ALICE, BOB};
use helpers::{post, setup, DATABASE_URL, REDIS_URL, SOCKET};
use tokio::timer::delay_for;

#[tokio::test]
async fn integration_tests() {
    setup();
    set_balance(REDIS_URL, ALICE.to_vec(), 100);
    let system_contract = include_bytes!("../src/wasm/ellipticoin_system_contract.wasm");
    tokio::spawn(ellipticoind::run(
        SOCKET.parse().unwrap(),
        &DATABASE_URL,
        REDIS_URL,
        system_contract.to_vec(),
    ));

    post(
        "/transactions",
        Transaction {
            contract_address: [vec![0; 32].to_vec(), b"System".to_vec()].concat(),
            sender: ALICE.to_vec(),
            nonce: 2,
            gas_limit: 100000000000000,
            function: "transfer".to_string(),
            arguments: vec![BOB.to_vec().into(), (50 as u64).into()],
        },
    )
    .await;
    delay_for(Duration::from_secs(1)).await;
    assert_eq!(get_balance(&BOB.to_vec()).await, 50);
}
