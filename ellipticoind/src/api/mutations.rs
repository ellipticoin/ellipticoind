use crate::api::blocks::process_received_block;
use crate::api::types::GraphQLPostBlockResult;
use crate::{
    api::{
        graphql::{Context, Error},
        helpers::validate_signature,
        types::{BlockResult, Bytes, Transaction},
    },
    constants::MINERS,
    helpers::run_transaction,
    models,
};
use ellipticoin::PublicKey;

pub struct Mutations;

#[juniper::graphql_object(
    Context = Context,
)]
impl Mutations {
    pub async fn post_transaction(
        _context: &Context,
        transaction: Bytes,
    ) -> Result<Transaction, Error> {
        let (transaction_request, _) = validate_signature(&transaction.0)?;
        let transaction = run_transaction(transaction_request).await;
        Ok(Transaction::from(transaction))
    }

    pub async fn post_block(
        _context: &Context,
        posted_block: Bytes,
    ) -> Result<GraphQLPostBlockResult, Error> {
        let ((block, txs), signer_address): (
            (models::block::Block, Vec<models::transaction::Transaction>),
            PublicKey,
        ) = validate_signature(&posted_block.0)?;

        let res: BlockResult =
            process_received_block(posted_block, block, txs, signer_address).await;
        Ok(GraphQLPostBlockResult::from(res))
    }

    pub async fn slash_winner(_context: &Context, block: Bytes) -> Result<bool, Error> {
        let ((message, winner), _): ((String, PublicKey), _) = validate_signature(&block.0)?;
        if message != "Slash" {
            return Err(Error("Message didn't start with \"Slash\"".to_string()));
        }
        if MINERS.current().await.address == winner {
            println!("Slash winner")
        }
        Ok(true)
    }
}
