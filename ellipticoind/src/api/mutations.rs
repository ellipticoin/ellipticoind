use crate::{
    api::{
        graphql::{Context, Error},
<<<<<<< HEAD
        types::Bytes,
    },
    transaction,
    transaction::SignedTransaction,
};

use juniper::FieldError;
use std::string::ToString;

=======
        helpers::validate_signature,
        types::{Bytes, Transaction},
    },
    constants::NEW_BLOCK_CHANNEL,
    helpers::run_transaction,
    models,
    state::current_miner,
};

>>>>>>> master
pub struct Mutations;

#[juniper::graphql_object(
    Context = Context,
)]
impl Mutations {
    pub async fn post_transaction(
        _context: &Context,
        transaction: Bytes,
<<<<<<< HEAD
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
=======
    ) -> Result<Transaction, Error> {
        let transaction_request = validate_signature(&transaction.0)?;
        let transaction = run_transaction(transaction_request).await;
        Ok(Transaction::from(transaction))
    }

    pub async fn post_block(_context: &Context, block: Bytes) -> Result<bool, Error> {
        let block: (models::block::Block, Vec<models::transaction::Transaction>) =
            validate_signature(&block.0)?;
        let state = block.clone().0.apply(block.1).await;
        println!("Applied block #{}", block.0.number);
        NEW_BLOCK_CHANNEL.0.send(state).await;

        Ok(true)
    }

    pub async fn slash_winner(_context: &Context, block: Bytes) -> Result<bool, Error> {
        let (message, winner): (String, [u8; 32]) = validate_signature(&block.0)?;
        if message != "Slash" {
            return Err(Error("Message didn't start with \"Slash\"".to_string()));
        }
        if current_miner().await.address == winner {
            println!("Slash winner")
        }
>>>>>>> master
        Ok(true)
    }
}
