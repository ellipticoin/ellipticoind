use crate::models;
use juniper::{ParseScalarResult, ParseScalarValue, Value};

#[derive(Clone, Debug)]
pub struct Token {
    pub id: Bytes,
    pub issuer: String,
    pub price: U64,
    pub balance: U64,
    pub total_supply: U64,
}

#[juniper::graphql_object]
impl Token {
    fn id(&self) -> Bytes {
        self.id.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn price(&self) -> U64 {
        self.price.clone()
    }

    fn balance(&self) -> U64 {
        self.balance.clone()
    }

    fn total_supply(&self) -> U64 {
        self.total_supply.clone()
    }
}

#[derive(Clone, Debug)]
pub struct LiquidityToken {
    pub id: Bytes,
    pub issuer: String,
    pub balance: U64,
    pub price: U64,
    pub total_supply: U64,
    pub pool_supply_of_token: U64,
    pub pool_supply_of_base_token: U64,
}

#[juniper::graphql_object]
impl LiquidityToken {
    fn id(&self) -> Bytes {
        self.id.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn balance(&self) -> U64 {
        self.balance.clone()
    }

    fn price(&self) -> U64 {
        self.price.clone()
    }

    fn total_supply(&self) -> U64 {
        self.total_supply.clone()
    }

    fn pool_supply_of_token(&self) -> U64 {
        self.pool_supply_of_token.clone()
    }

    fn pool_supply_of_base_token(&self) -> U64 {
        self.pool_supply_of_base_token.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub number: U32,
    pub transactions: Vec<Transaction>,
    pub sealed: bool,
    pub memory_changeset_hash: Bytes,
    pub storage_changeset_hash: Bytes,
}

#[juniper::graphql_object]
impl Block {
    fn number(&self) -> U32 {
        self.number.clone()
    }

    fn transactions(&self) -> Vec<Transaction> {
        self.transactions.clone()
    }

    fn sealed(&self) -> bool {
        self.sealed
    }

    fn memory_changeset_hash(&self) -> Bytes {
        self.memory_changeset_hash.clone()
    }

    fn storage_changeset_hash(&self) -> Bytes {
        self.storage_changeset_hash.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Transaction {
    pub id: U32,
    pub network_id: U64,
    pub block_number: U32,
    pub position: U32,
    pub contract: String,
    pub sender: Bytes,
    pub nonce: U32,
    pub function: String,
    pub arguments: Bytes,
    pub return_value: Bytes,
    pub raw: Bytes,
}

#[juniper::graphql_object]
impl Transaction {
    fn id(&self) -> U32 {
        self.id.clone()
    }

    fn network_id(&self) -> U64 {
        self.network_id.clone()
    }

    fn block_number(&self) -> U32 {
        self.block_number.clone()
    }

    fn position(&self) -> U32 {
        self.position.clone()
    }

    fn contract(&self) -> String {
        self.contract.clone()
    }

    fn sender(&self) -> Bytes {
        self.sender.clone()
    }

    fn nonce(&self) -> U32 {
        self.nonce.clone()
    }

    fn function(&self) -> String {
        self.function.clone()
    }

    fn arguments(&self) -> Bytes {
        self.arguments.clone()
    }

    fn return_value(&self) -> Bytes {
        self.return_value.clone()
    }

    fn raw(&self) -> Bytes {
        self.raw.clone()
    }
}

impl From<models::Transaction> for Transaction {
    fn from(transaction: models::Transaction) -> Self {
        Self {
            id: U32(transaction.id as u32),
            network_id: U64(transaction.network_id as u64),
            nonce: U32(transaction.nonce as u32),
            position: U32(transaction.position as u32),
            block_number: U32(transaction.block_number as u32),
            function: transaction.function,
            sender: Bytes(transaction.sender),
            contract: transaction.contract,
            arguments: transaction.arguments.into(),
            return_value: Bytes(transaction.return_value),
            raw: Bytes(transaction.raw),
        }
    }
}
impl From<(models::Block, Vec<models::Transaction>)> for Block {
    fn from(block: (models::Block, Vec<models::Transaction>)) -> Block {
        Self {
            number: U32(block.0.number as u32),
            sealed: block.0.sealed,
            memory_changeset_hash: Bytes(block.0.memory_changeset_hash),
            storage_changeset_hash: Bytes(block.0.storage_changeset_hash),
            transactions: block
                .1
                .into_iter()
                .map(Transaction::from)
                .collect::<Vec<Transaction>>(),
        }
    }
}

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

#[derive(Clone, juniper::GraphQLInputObject)]
pub struct TokenId {
    pub id: Bytes,
    pub issuer: String,
}

impl From<TokenId> for ellipticoin::Token {
    fn from(token_id: TokenId) -> Self {
        Self {
            id: token_id.id.0.into(),
            issuer: ellipticoin::Address::Contract(token_id.issuer),
        }
    }
}

#[derive(Clone, Debug)]
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
