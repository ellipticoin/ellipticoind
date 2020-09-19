use crate::{
    api::{
        graphql::{Context, Error},
        types::{Bytes, Transaction},
    },
    models,
    transaction::TransactionRequest,
};
use ed25519_zebra::{SigningKey, VerificationKey};
use futures::channel::oneshot;
use juniper::graphql_value;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

pub struct Mutations;

impl juniper::IntoFieldError for Error {
    fn into_field_error(self) -> juniper::FieldError {
        juniper::FieldError::new(
            self.to_string(),
            graphql_value!({
                "type": "Error"
            }),
        )
    }
}

#[juniper::graphql_object(
    Context = Context,
)]
impl Mutations {
    pub async fn post_transaction(
        context: &Context,
        transaction: Bytes,
    ) -> Result<Transaction, Error> {
        let mut sign1 = serde_cose::from_slice(&transaction.0).map_err(|e| Error(e.to_string()))?;
        validate_signature(&mut sign1)?;
        let transaction = parse_transaction(&sign1)?;
        let sender = &context.sender;
        let (responder, response) = oneshot::channel();
        sender
            .send(crate::api::Message::Transaction(
                transaction.clone(),
                responder,
            ))
            .await;

        let completed_transaction = response.await.unwrap();

        Ok(completed_transaction.into())
    }

    pub async fn post_block(context: &Context, block: Bytes) -> Result<bool, Error> {
        let mut sign1 = serde_cose::from_slice(&block.0).map_err(|e| Error(e.to_string()))?;
        validate_signature(&mut sign1)?;
        let block: (models::block::Block, Vec<models::transaction::Transaction>) =
            serde_cbor::from_slice(&sign1.payload).map_err(|e| Error(e.to_string()))?;

        let sender = &context.sender;
        sender.send(crate::api::Message::Block(block.clone())).await;

        Ok(true)
    }
}

use serde_cbor::value::from_value;
pub fn parse_transaction(cose: &serde_cose::Sign1) -> Result<TransactionRequest, Error> {
    let transaction: HashMap<String, serde_cbor::Value> =
        serde_cbor::from_slice(&cose.payload).unwrap();
    Ok(TransactionRequest {
        nonce: from_value(transaction.get("nonce").unwrap().clone()).unwrap(),
        arguments: from_value(transaction.get("arguments").unwrap().clone()).unwrap(),
        function: from_value(transaction.get("function").unwrap().clone()).unwrap(),
        contract: from_value(transaction.get("contract").unwrap().clone()).unwrap(),
        network_id: from_value(transaction.get("network_id").unwrap().clone()).unwrap(),
        sender: cose.kid()[..].try_into().unwrap(),
    })
}
pub fn validate_signature(sign1: &mut serde_cose::Sign1) -> Result<(), Error> {
    let key = serde_cose::Key::from(
        VerificationKey::try_from(<[u8; 32]>::try_from(&sign1.kid()[..]).unwrap()).unwrap(),
    );
    if key.verify(sign1) {
        Ok(())
    } else {
        Err(Error("invalid signature".to_string()))
    }
}
