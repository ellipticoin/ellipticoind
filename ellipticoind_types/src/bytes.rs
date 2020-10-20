use core::{
    cmp::{self},
    fmt::{self, Debug},
};
pub use types::*;
use wasm_rpc::serde::{
    de::{self, Deserializer, SeqAccess, Visitor},
    ser::Serializer,
    Deserialize, Serialize,
};

struct BytesVisitor;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bytes(pub Vec<u8>);
impl Bytes {
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
    pub fn from<T: Into<Vec<u8>>>(bytes: T) -> Self {
        Bytes(bytes.into())
    }
}
impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("byte array")
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Bytes, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let len = cmp::min(visitor.size_hint().unwrap_or(0), 4096);
        let mut bytes = Vec::with_capacity(len);

        while let Some(b) = visitor.next_element()? {
            bytes.push(b);
        }

        Ok(Bytes::from(bytes))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Bytes, E>
    where
        E: de::Error,
    {
        Ok(Bytes::from(v))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Bytes, E>
    where
        E: de::Error,
    {
        Ok(Bytes::from(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Bytes, E>
    where
        E: de::Error,
    {
        Ok(Bytes::from(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Bytes, E>
    where
        E: de::Error,
    {
        Ok(Bytes::from(v.as_bytes().to_vec()))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_byte_buf(BytesVisitor)
    }
}
impl Into<Bytes> for Vec<u8> {
    fn into(self) -> Bytes {
        Bytes(self)
    }
}
