// pub async fn sign<S: Serialize>(payload: S) -> Sign1 {
//     let mut sign1 = Sign1::new(payload, verification_key().to_vec());
//     sign1.sign(signing_key());
//     sign1
// }
//
// pub fn base64_encode<S: Serialize>(payload: S) -> String {
//     base64::encode(&serde_cbor::to_vec(&payload).unwrap())
// }
