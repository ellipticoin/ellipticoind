use crate::{
    api::{
        graphql::{Context, Error},
        helpers::validate_signature,
        types::{Bytes, Transaction},
    },
    constants::{CURRENT_MINER_CHANNEL, MINERS},
    helpers::run_transaction,
    models,
};

pub struct Mutations;

#[juniper::graphql_object(
    Context = Context,
)]
impl Mutations {
    pub async fn post_transaction(
        _context: &Context,
        transaction: Bytes,
    ) -> Result<Transaction, Error> {
        let transaction_request = validate_signature(&transaction.0)?;
        let transaction = run_transaction(transaction_request).await;
        Ok(Transaction::from(transaction))
    }

    pub async fn post_block(_context: &Context, block: Bytes) -> Result<bool, Error> {
        let block: (models::block::Block, Vec<models::transaction::Transaction>) =
            validate_signature(&block.0)?;
        let miner = block.0.apply(block.1).await;
        CURRENT_MINER_CHANNEL.0.send(miner).await;

        Ok(true)
    }

    pub async fn slash_winner(_context: &Context, block: Bytes) -> Result<bool, Error> {
        let (message, winner): (String, [u8; 32]) = validate_signature(&block.0)?;
        if message != "Slash" {
            return Err(Error("Message didn't start with \"Slash\"".to_string()));
        }
        if MINERS.current().await.address == winner {
            println!("Slash winner")
        }
        Ok(true)
    }
}
