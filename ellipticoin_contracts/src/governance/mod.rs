use crate::{
    contract::{self, Contract},
    token::Token,
    Action, Ellipticoin,
};
use anyhow::{anyhow, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Vote {
    For,
    Against,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub subtitle: String,
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
    pub fn create_proposal<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        title: String,
        subtitle: String,
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
            title,
            subtitle,
            actions,
            votes,
        };
        proposals.push(proposal);
        Self::increment_proposal_id_counter(db);
        Self::set_proposals(db, proposals);
        Ok(())
    }

    pub fn vote<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        proposal_id: u64,
        vote: Vote,
    ) -> Result<()> {
        println!("{} voted", hex::encode(sender));
        let mut proposals = Self::get_proposals(db);
        let index = proposals
            .iter()
            .cloned()
            .position(|proposal| proposal.id == proposal_id)
            .ok_or(anyhow!("Proposal {} not found", proposal_id))?;
        proposals[index].votes.insert(sender, vote);
        if proposals[index]
            .votes
            .iter()
            .map(|(address, vote)| {
                if *vote == Vote::For {
                    Token::get_balance(db, *address, Ellipticoin::address())
                } else {
                    0
                }
            })
            .sum::<u64>()
            * 100
            / Token::get_total_supply(db, Ellipticoin::address())
            > 50
        {
            for action in &proposals[index].actions {
                action.run(db, Self::address())?;
            }
            println!("ratified!");
        }
        Self::set_proposals(db, proposals);
        Ok(())
    }

    fn increment_proposal_id_counter<B: Backend>(db: &mut Db<B>) -> u64 {
        let proposal_id_counter = Self::get_proposal_id_counter(db) + 1;
        Self::set_proposal_id_counter(db, proposal_id_counter);
        proposal_id_counter
    }
}

#[cfg(test)]
mod tests {
    use super::{Governance, Proposal, Vote};
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
        votes.insert(ALICE, Vote::For);
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
            }
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
        Governance::vote(&mut db, BOB, 0, Vote::For).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 1);
    }
}
