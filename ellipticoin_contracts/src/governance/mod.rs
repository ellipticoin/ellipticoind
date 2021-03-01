use crate::{
    constants::MS,
    contract::{self, Contract},
    token::Token,
    Action,
};
use anyhow::{anyhow, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{Address, DB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::transaction::Run;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Vote {
    For,
    Against,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub content: String,
    pub actions: Vec<Action>,
    pub votes: HashMap<Address, Vote>,
}
pub struct Governance;

impl Contract for Governance {
    const NAME: contract::Name = contract::Name::Governance;
}

db_accessors!(Governance {
    proposals() -> Vec<Proposal>;
    proposal_id_counter() -> u64;
});

impl Governance {
    pub fn create_proposal<D: ellipticoin_types::DB>(
        db: &mut D,
        sender: Address,
        content: String,
        actions: Vec<Action>,
    ) -> Result<()> {
        let mut proposals = Self::get_proposals(db);
        let mut votes = HashMap::new();
        votes.insert(sender, Vote::For);
        let proposal = Proposal {
            id: Self::get_proposal_id_counter(db),
            proposer: sender,
            content,
            actions,
            votes,
        };
        proposals.push(proposal);
        Self::increment_proposal_id_counter(db);
        Self::set_proposals(db, proposals);
        Ok(())
    }

    pub fn vote<D: ellipticoin_types::DB>(
        db: &mut D,
        sender: Address,
        proposal_id: u64,
        vote: Vote,
    ) -> Result<()> {
        let proposals = Self::get_proposals(db);
        let index = proposals
            .iter()
            .cloned()
            .position(|proposal| proposal.id == proposal_id)
            .ok_or(anyhow!("Proposal {} not found", proposal_id))?;
        let mut proposal = proposals[index].clone();
        proposal.votes.insert(sender, vote);
        if proposal
            .votes
            .iter()
            .map(|(address, vote)| {
                if *vote == Vote::For {
                    Token::get_balance(db, *address, MS)
                } else {
                    0
                }
            })
            .sum::<u64>()
            * 100
            / Token::get_total_supply(db, MS)
            > 50
        {
            for action in proposal.actions {
                action.run(db, Self::address())?;
            }

        }
        Ok(())
    }

    fn increment_proposal_id_counter<D: DB>(db: &mut D) -> u64 {
        let proposal_id_counter = Self::get_proposal_id_counter(db) + 1;
        Self::set_proposal_id_counter(db, proposal_id_counter);
        proposal_id_counter
    }
}

#[cfg(test)]
mod tests {
    use super::{Governance, Proposal, Vote};
    use crate::{constants::MS, Action, Token};
    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB, CAROL},
            tokens::APPLES,
        },
        test_db::TestDB,
    };
    use std::collections::HashMap;
    use crate::contract::Contract;

    #[test]
    fn create_proposal() {
        let mut db = TestDB::new();
        let actions = vec![Action::Transfer(1, APPLES, ALICE)];
        let mut votes = HashMap::new();
        votes.insert(ALICE, Vote::For);
        Token::mint(&mut db, 1, MS, ALICE);
        Token::mint(&mut db, 1, MS, BOB);
        Token::mint(&mut db, 1, MS, CAROL);

        Governance::create_proposal(&mut db, ALICE, "Pay Alice".to_string(), actions.clone())
            .unwrap();
        assert_eq!(
            Governance::get_proposals(&mut db)[0],
            Proposal {
                id: 0,
                proposer: ALICE,
                content: "Pay Alice".to_string(),
                actions,
                votes,
            }
        );
    }

    #[test]
    fn vote() {
        let mut db = TestDB::new();
        let actions = vec![Action::Transfer(1, APPLES, ALICE)];
        Token::mint(&mut db, 1, APPLES, Governance::address());
        Token::mint(&mut db, 1, MS, ALICE);
        Token::mint(&mut db, 1, MS, BOB);
        Token::mint(&mut db, 1, MS, CAROL);

        Governance::create_proposal(&mut db, ALICE, "Pay Alice".to_string(), actions.clone())
            .unwrap();
        Governance::vote(&mut db, BOB, 0, Vote::For).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 1);
    }
}
