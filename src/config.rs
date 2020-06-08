use serde::{Deserialize, Deserializer};

pub struct Thing {}

#[derive(Deserialize, Debug)]
pub struct Bootnode {
    pub host: String,
    #[serde(deserialize_with = "decode_base64")]
    public_key: Vec<u8>,
}

pub fn decode_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| base64::decode(&string).map_err(|err| Error::custom(err.to_string())))
}

pub fn bootnodes() -> Vec<Bootnode> {
    serde_yaml::from_str(&include_str!("bootnodes.yaml")).unwrap()
}
