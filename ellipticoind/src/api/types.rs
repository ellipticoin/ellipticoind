use juniper::{ParseScalarResult, ParseScalarValue, Value};
use std::convert::TryInto;

#[derive(Clone, Debug)]
pub struct Bridge {
    pub address: Address,
    pub signers: Vec<Bytes>,
}

#[juniper::graphql_object]
impl Bridge {
    fn address(&self) -> Address {
        self.address.clone()
    }

    fn signers(&self) -> Vec<Bytes> {
        self.signers.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub address: Address,
    pub interest_rate: Option<U64>,
    pub price: U64,
    pub balance: U64,
    pub total_supply: U64,
}

#[juniper::graphql_object]
impl Token {
    fn address(&self) -> Address {
        self.address.clone()
    }

    fn interest_rate(&self) -> Option<U64> {
        self.interest_rate.clone()
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
    pub token_address: Address,
    pub balance: U64,
    pub total_supply: U64,
    pub pool_supply_of_token: U64,
    pub pool_supply_of_base_token: U64,
}

#[juniper::graphql_object]
impl LiquidityToken {
    fn token_address(&self) -> Address {
        self.token_address.clone()
    }

    fn balance(&self) -> U64 {
        self.balance.clone()
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
pub struct Order {
    pub id: U64,
    pub order_type: String,
    pub amount: U64,
    pub token: Address,
    pub price: U64,
}

#[juniper::graphql_object]
impl Order {
    fn id(&self) -> U64 {
        self.id.clone()
    }

    fn order_type(&self) -> String {
        self.order_type.clone()
    }

    fn token(&self) -> Address {
        self.token.clone()
    }

    fn amount(&self) -> U64 {
        self.amount.clone()
    }

    fn price(&self) -> U64 {
        self.price.clone()
    }
}
#[derive(Clone, Debug)]
pub struct Proposal {
    pub id: U64,
    pub proposer: Address,
    pub title: String,
    pub subtitle: String,
    pub content: String,
    pub actions: Vec<Bytes>,
    pub votes: Vec<Vote>,
    pub result: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Vote {
    pub choice: String,
    pub voter: Address,
    pub weight: U64,
}

#[juniper::graphql_object]
impl Vote {
    fn choice(&self) -> String {
        self.choice.clone()
    }

    fn voter(&self) -> Address {
        self.voter.clone()
    }

    fn weight(&self) -> U64 {
        self.weight.clone()
    }
}
#[juniper::graphql_object]
impl Proposal {
    fn id(&self) -> U64 {
        self.id.clone()
    }

    fn token_address(&self) -> Address {
        self.proposer.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn subtitle(&self) -> String {
        self.subtitle.clone()
    }

    fn content(&self) -> String {
        self.content.clone()
    }

    fn actions(&self) -> Vec<Bytes> {
        self.actions.clone()
    }

    fn votes(&self) -> Vec<Vote> {
        self.votes.clone()
    }

    fn result(&self) -> Option<String> {
        self.result.clone()
    }
}
pub struct RedeemRequest {
    pub id: U64,
    pub sender: Address,
    pub token: Address,
    pub amount: U64,
    pub expiration_block_number: U64,
    pub signature: Bytes,
}

#[juniper::graphql_object]
impl RedeemRequest {
    fn id(&self) -> U64 {
        self.id.clone()
    }

    fn sender(&self) -> Address {
        self.sender.clone()
    }

    fn token(&self) -> Address {
        self.token.clone()
    }

    fn amount(&self) -> U64 {
        self.amount.clone()
    }

    fn expiration_block_number(&self) -> U64 {
        self.expiration_block_number.clone()
    }

    fn signature(&self) -> Bytes {
        self.signature.clone()
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
    pub transaction_number: U32,
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

    fn transaction_number(&self) -> U32 {
        self.transaction_number.clone()
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

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Address(pub ellipticoin_types::Address);

impl From<Address> for ellipticoin_types::Address {
    fn from(address: Address) -> Self {
        address.0
    }
}
#[juniper::graphql_scalar(description = "Address")]
impl<S> GraphQLScalar for Address
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(base64::encode(&self.0))
    }

    fn from_input_value(v: &InputValue) -> Option<Address> {
        v.as_scalar_value()
            .and_then(|v| v.as_str())
            .map(|v| base64::decode(v))
            .and_then(Result::ok)
            .map(|inner| inner[..20].try_into())
            .and_then(Result::ok)
            .map(|inner| Address(ellipticoin_types::Address(inner)))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}

impl From<ellipticoin_types::Address> for Address {
    fn from(bytes: ellipticoin_types::Address) -> Self {
        Self(bytes)
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
