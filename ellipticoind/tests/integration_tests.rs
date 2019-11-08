extern crate ellipticoind;
use vm::Transaction;
mod helpers;
use helpers::post;

#[tokio::test]
async fn integration_tests() {
    tokio::spawn(ellipticoind::run("127.0.0.1:3030".parse().unwrap()));
    post(
        "http://localhost:3030/transactions",
        Transaction {
            contract_address: vec![1, 2, 3],
            sender: vec![1, 2, 3],
            nonce: 2,
            gas_limit: 99,
            function: "test".to_string(),
            arguments: vec![],
        },
    )
    .await;
}
