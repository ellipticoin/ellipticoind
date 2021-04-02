mod validations;

use crate::constants::MS;
use crate::{
    charge,
    contract::{self, Contract},
    token::Token,
    Action, Ellipticoin,
};
use anyhow::{Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Vote {
    voter: Address,
    choice: Choice,
    weight: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Choice {
    For,
    Against,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: usize,
    pub proposer: Address,
    pub title: String,
    pub subtitle: String,
    pub content: String,
    pub actions: Vec<Action>,
    pub result: Option<Choice>,
    pub votes: HashMap<Address, Choice>,
}
pub struct Governance;

impl Contract for Governance {
    const NAME: contract::Name = contract::Name::Governance;
}

db_accessors!(Governance {
    proposals() -> Vec<Proposal>;
    proposal_id_counter() -> usize;
});

impl Governance {
    pub fn create_proposal<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        title: String,
        subtitle: String,
        content: String,
        actions: Vec<Action>,
    ) -> Result<()> {
        Self::validatate_minimum_proposal_theshold(db, sender)?;
        let mut proposals = Self::get_proposals(db);
        let mut votes = HashMap::new();
        votes.insert(sender, Choice::For);
        let proposal = Proposal {
            id: Self::get_proposal_id_counter(db),
            proposer: sender,
            content,
            title,
            subtitle,
            actions,
            votes,
            result: None,
        };
        proposals.push(proposal);
        Self::increment_proposal_id_counter(db);
        Self::set_proposals(db, proposals);
        Ok(())
    }

    pub fn vote<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        proposal_id: usize,
        vote: Choice,
    ) -> Result<()> {
        let balance = Token::get_balance(db, sender, MS);
        charge!(db, sender, MS, balance)?;
        let mut proposals = Self::get_proposals(db);
        proposals[proposal_id].votes.insert(sender, vote);
        let votes_for = Self::tally(db, &proposals[proposal_id].votes, Choice::For);
        let votes_against = Self::tally(db, &proposals[proposal_id].votes, Choice::Against);
        if votes_for * 100 / Token::get_total_supply(db, Ellipticoin::address()) > 50 {
            for action in &proposals[proposal_id].actions {
                action.run(db, Self::address())?;
            }
            proposals[proposal_id].result = Some(Choice::For);
        } else if votes_against * 100 / Token::get_total_supply(db, Ellipticoin::address()) > 50 {
            proposals[proposal_id].result = Some(Choice::Against);
        }
        Self::set_proposals(db, proposals);
        Ok(())
    }


    pub fn tally<B: Backend>(
        db: &mut Db<B>,
        votes: &HashMap<Address, Choice>,
        vote_to_tally: Choice,
    ) -> u64 {
        votes
            .iter()
            .map(|(address, vote)| {
                if *vote == vote_to_tally {
                    Token::get_balance(db, *address, Ellipticoin::address())
                } else {
                    0
                }
            })
            .sum()
    }

    fn increment_proposal_id_counter<B: Backend>(db: &mut Db<B>) -> usize {
        let proposal_id_counter = Self::get_proposal_id_counter(db) + 1;
        Self::set_proposal_id_counter(db, proposal_id_counter);
        proposal_id_counter
    }
}

#[cfg(test)]
mod tests {
    use super::{Governance, Proposal, Choice};
    use crate::{constants::MS, contract::Contract, Action, Token};
    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB, CAROL},
            tokens::APPLES,
        },
        new_db,
    };
    use std::collections::HashMap;

    #[test]
    fn create_proposal() {
        let mut db = new_db();
        let actions = vec![Action::Pay(ALICE, 1, APPLES)];
        let mut votes = HashMap::new();
        votes.insert(ALICE, Choice::For);
        Token::mint(&mut db, 1, MS, ALICE);
        Token::mint(&mut db, 1, MS, BOB);
        Token::mint(&mut db, 1, MS, CAROL);

        Governance::create_proposal(
            &mut db,
            ALICE,
            "Pay Alice".to_string(),
            "Test Subtitle".to_string(),
            "Test Content".to_string(),
            actions.clone(),
        )
        .unwrap();
        assert_eq!(
            Governance::get_proposals(&mut db)[0],
            Proposal {
                id: 0,
                proposer: ALICE,
                title: "Pay Alice".to_string(),
                subtitle: "Test Subtitle".to_string(),
                content: "Test Content".to_string(),
                actions,
                votes,
                result: None,
            }
        );
    }

    #[test]
    fn create_proposal_with_insufficient_moonshine() {
        let mut db = new_db();
        let actions = vec![];
        let mut votes = HashMap::new();
        votes.insert(ALICE, Choice::For);
        Token::mint(&mut db, 1, MS, ALICE);
        Token::mint(&mut db, 100, MS, BOB);

        assert_eq!(
            Governance::create_proposal(
                &mut db,
                ALICE,
                "Pay Alice".to_string(),
                "Test Subtitle".to_string(),
                "Test Content".to_string(),
                actions.clone(),
            )
            .err()
            .unwrap()
            .to_string(),
            "5 % of total tokens in circulation required to create proposals"
        );
    }

    #[test]
    fn vote() {
        let mut db = new_db();
        let actions = vec![Action::Pay(ALICE, 1, APPLES)];
        Token::mint(&mut db, 1, APPLES, Governance::address());
        Token::mint(&mut db, 1, MS, ALICE);
        Token::mint(&mut db, 1, MS, BOB);
        Token::mint(&mut db, 1, MS, CAROL);

        Governance::create_proposal(
            &mut db,
            ALICE,
            "Pay Alice".to_string(),
            "Test Subtitle".to_string(),
            "Test Content".to_string(),
            actions.clone(),
        )
        .unwrap();
        Governance::vote(&mut db, BOB, 0, Choice::For).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 1);
    }
}
