use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use vm::Transaction;
use warp::reply::Response;
pub static NEXT_BLOCK_NUMBER: AtomicUsize = AtomicUsize::new(1);

pub fn block_number() -> Response {
    let nonce = NEXT_BLOCK_NUMBER.load(Ordering::Relaxed);
    let transaction = Transaction {
        contract_address: vec![1, 2, 3],
        sender: vec![1, 2, 3],
        nonce: nonce as u64,
        gas_limit: 99,
        function: "test".to_string(),
        arguments: vec![],
    };
    Response::new(serde_cbor::to_vec(&transaction).unwrap().into())
}
