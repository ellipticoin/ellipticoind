use crate::{
    api::{
        graphql::{Context, Error},
        types::Bytes,
    },
    transaction,
    transaction::SignedTransaction,
};

use juniper::FieldError;
use std::string::ToString;

pub struct Mutations;

#[juniper::graphql_object(
    Context = Context,
)]
impl Mutations {
    pub async fn post_transaction(
        _context: &Context,
        transaction: Bytes,
    ) -> Result<Option<String>, FieldError> {
        let signed_transaction: ellipticoin_peerchain_ethereum::SignedTransaction =
            serde_cbor::from_slice(&transaction.0).map_err(|err| Error(err.to_string()))?;
        Ok(
            transaction::dispatch(SignedTransaction::Ethereum(signed_transaction))
                .await
                .map_err(|err| err.to_string())
                .err(),
        )
    }

    pub async fn post_block(_context: &Context, _block: Bytes) -> Result<bool, FieldError> {
        Ok(true)
    }
}
