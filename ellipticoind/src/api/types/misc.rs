use juniper::parser::ScalarToken;
use juniper::{InputValue, ParseScalarResult, ParseScalarValue, ScalarValue, Value};

#[derive(Clone, Debug)]
pub struct U64(pub u64);

impl From<U64> for String {
    fn from(n: U64) -> Self {
        n.0.to_string()
    }
}

impl From<u64> for U64 {
    fn from(n: u64) -> Self {
        U64(n)
    }
}

#[derive(Clone, Debug)]
pub struct U32(pub u32);
impl From<U32> for String {
    fn from(n: U32) -> Self {
        n.0.to_string()
    }
}

impl From<u32> for U32 {
    fn from(n: u32) -> Self {
        U32(n)
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Bytes(pub Vec<u8>);

impl From<Bytes> for Vec<u8> {
    fn from(bytes: Bytes) -> Self {
        bytes.0
    }
}
#[juniper::graphql_scalar(description = "Bytes")]
impl<S> GraphQLScalar for Bytes
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(base64::encode(&self.0))
    }

    fn from_input_value(v: &InputValue) -> Option<Bytes> {
        v.as_scalar_value()
            .and_then(|v| v.as_str())
            .map(|v| base64::decode(v))
            .and_then(Result::ok)
            .map(|inner| Bytes(inner))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(vec: Vec<u8>) -> Self {
        Bytes(vec)
    }
}

#[juniper::graphql_scalar(description = "U64")]
impl<S> GraphQLScalar for U64
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.0.to_string())
    }

    fn from_input_value(v: &InputValue) -> Option<U64> {
        v.as_scalar_value()
            .and_then(|v| v.as_str())
            .map(|v| v.parse())
            .and_then(Result::ok)
            .map(|inner| U64(inner))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}

#[juniper::graphql_scalar(description = "U32")]
impl<S> GraphQLScalar for U32
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.0.to_string())
    }

    fn from_input_value(v: &InputValue) -> Option<U32> {
        v.as_scalar_value()
            .and_then(|v| v.as_str())
            .map(|v| v.parse())
            .and_then(Result::ok)
            .map(|inner| U32(inner))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}
