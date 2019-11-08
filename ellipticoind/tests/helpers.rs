use serde::de::DeserializeOwned;
use serde::Serialize;

pub async fn _get<D: DeserializeOwned>(path: &str) -> D {
    let response = reqwest::get(path).await.unwrap();
    let bytes = response.bytes().await.unwrap();
    serde_cbor::from_slice::<D>(&bytes).unwrap()
}

pub async fn post<S: Serialize>(path: &str, payload: S) {
    let response = reqwest::Client::new()
        .post(path)
        .body(serde_cbor::to_vec(&payload).unwrap())
        .send()
        .await
        .unwrap();
    println!("{:?}", response.bytes().await.unwrap());
}
