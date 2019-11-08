use vm::Transaction;
use warp::http::StatusCode;
use warp::reply::Reply;

pub fn create(transaction: Transaction) -> impl Reply {
    println!("{:?}", transaction);

    StatusCode::CREATED
}
