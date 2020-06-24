use crate::config::get_redis_connection;
use crate::vm::{redis::Commands, Transaction};
pub struct TransactionPool;

impl TransactionPool {
    pub fn add(transaction: &Transaction) {
        let mut redis = get_redis_connection();
        redis
            .rpush::<&str, Vec<u8>, ()>(
                "transactions::pending",
                serde_cbor::to_vec(transaction).unwrap(),
            )
            .unwrap();
    }
}
