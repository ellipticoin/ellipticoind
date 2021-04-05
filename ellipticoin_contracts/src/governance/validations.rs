use crate::{
    constants::{BASE_FACTOR, MS},
    Governance, Token,
};
use anyhow::{bail, Result};
use ellipticoin_types::{db::Backend, Address, Db};

const MINIMUM_PROPOSAL_THRESHOLD: u64 = 50000;

impl Governance {
    pub fn validatate_minimum_proposal_theshold<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
    ) -> Result<()> {
        let balance = Token::get_balance(db, sender, MS);
        let total_supply = Token::get_total_supply(db, MS);
        if balance > total_supply * MINIMUM_PROPOSAL_THRESHOLD / BASE_FACTOR {
            Ok(())
        } else {
            bail!("5 % of total tokens in circulation required to create proposals")
        }
    }
}
