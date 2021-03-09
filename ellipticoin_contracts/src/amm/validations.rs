use super::AMM;
use anyhow::{bail, Result};
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
impl AMM {
    pub fn validate_pool_does_not_exist<B: Backend>(db: &mut Db<B>, token: Address) -> Result<()> {
        if Self::get_pool_supply_of_base_token(db, token) != 0 {
            bail!("Pool already exisits: {}", hex::encode(token))
        } else {
            Ok(())
        }
    }

    pub fn validate_pool_exists<B: Backend>(db: &mut Db<B>, token: Address) -> Result<()> {
        if Self::get_pool_supply_of_token(db, token.clone()) > 0 {
            Ok(())
        } else {
            bail!("Pool does not exisit: {}", hex::encode(token))
        }
    }

    pub fn validate_slippage(
        minimum_output_token_amount: u64,
        output_token_amount: u64,
    ) -> Result<()> {
        if output_token_amount >= minimum_output_token_amount {
            Ok(())
        } else {
            bail!("Maximum slippage exceeded")
        }
    }
}
