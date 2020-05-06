use super::State;
use http_service::Body;
use tide::Response;
use vm::redis::Commands;

pub async fn show(req: tide::Request<State>) -> Response {
    let key: String = req.param("key").unwrap();
    let mut redis = req.state().redis.get_connection().unwrap();
    let value = redis
        .get::<Vec<u8>, Vec<u8>>(base64::decode_config(&key, base64::URL_SAFE).unwrap())
        .unwrap();

    //  use vm::state::db_key;
    // println!("{:?}", redis
    //     .get::<_, Vec<u8>>(
    //         db_key(
    //             &crate::constants::TOKEN_CONTRACT,
    //             &[vec![crate::start_up::Namespace::Balances as u8], crate::constants::GENISIS_ADRESS.to_vec()].concat(),
    //             )
    //     ));
    Response::new(200).body(Body::from(serde_cbor::to_vec(&value).unwrap()))
}
