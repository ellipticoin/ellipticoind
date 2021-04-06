use crate::{
    constants::{BASE_FACTOR, MS},
    governance::Proposal,
    Governance, Token,
};
use anyhow::{bail, Result};
use ellipticoin_types::{db::Backend, Address, Db};

const MINIMUM_PROPOSAL_THRESHOLD: u64 = 50000;

impl Governance {
    pub fn validate_balance<B: Backend>(db: &mut Db<B>, sender: Address) -> Result<()> {
        let balance = Token::get_balance(db, sender, MS);
        if balance > 0 {
            Ok(())
        } else {
            bail!("Moonshine balance greater that zero required for voting")
        }
    }

    pub fn validate_proposal_is_open(proposal: &Proposal) -> Result<()> {
        if proposal.result.is_none() {
            Ok(())
        } else {
            bail!("Voting on this proposal has closed")
        }
    }

    pub fn validate_minimum_proposal_theshold<B: Backend>(
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
