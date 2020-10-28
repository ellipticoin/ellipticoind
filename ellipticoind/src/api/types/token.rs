use crate::api::types::misc::{Bytes, U64};

#[derive(Clone, Debug)]
pub struct Token {
    pub id: Bytes,
    pub issuer: String,
    pub balance: U64,
    pub price: U64,
}

#[juniper::graphql_object]
impl Token {
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
}

#[derive(Clone, juniper::GraphQLInputObject)]
pub struct TokenId {
    pub id: Bytes,
    pub issuer: String,
}
