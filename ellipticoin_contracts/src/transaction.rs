use crate::governance::Vote;
use crate::{Bridge, Ellipticoin, Governance, System, Token, AMM};
use anyhow::{bail, Result};
use ellipticoin_types::{Address, DB};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transaction<R: Run> {
    pub transaction_number: u64,
    pub network_id: u64,
    pub action: R,
}

impl Transaction<Action> {
    pub fn run<D: DB>(&self, db: &mut D, sender: Address) -> Result<()> {
        self.action.run(db, sender)
    }
}

pub trait Run {
    fn run<D: DB>(&self, db: &mut D, address: Address) -> Result<()>;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Action {
    CreateProposal(String, String, String, Vec<Action>),
    StartMining(String, [u8; 32]),
    Seal([u8; 32]),
    CreateRedeemRequest(u64, Address),
    SignRedeemRequest(u64, u64, Vec<u8>),
    Migrate([u8; 32], Vec<u8>),
    Pay(Address, u64, Address),
    CreatePool(u64, Address, u64),
    AddLiquidity(u64, Address),
    RemoveLiquidity(u64, Address),
    Trade(u64, Address, u64, Address),
    Harvest(),
    Redeem(u64, Address),
    Vote(u64, Vote),
}
impl Run for Action {
    fn run<D: DB>(&self, db: &mut D, sender: Address) -> Result<()> {
        System::increment_transaction_number(db, sender);
        let result = match &self {
            Action::CreateProposal(title, subtitle, content, actions) => {
                Governance::create_proposal(
                    db,
                    sender,
                    title.to_string(),
                    subtitle.to_string(),
                    content.to_string(),
                    actions.to_vec(),
                )
            }
            Action::StartMining(host, onion_skin) => {
                Ellipticoin::start_mining(db, sender, host.to_string(), *onion_skin)
            }
            Action::Seal(onion_skin) => Ellipticoin::seal(db, sender, *onion_skin),
            Action::SignRedeemRequest(redeem_id, expiration_block_number, signature) => {
                Bridge::sign_redeem_request(
                    db,
                    *redeem_id,
                    *expiration_block_number,
                    signature.to_vec(),
                )
            }
            Action::Migrate(legacy_address, legacy_signature) => {
                Token::migrate(db, sender, *legacy_address, legacy_signature.to_vec())
            }
            Action::Pay(amount, token, recipient) => {
                Token::transfer(db, sender, *amount, *token, *recipient)
            }
            Action::CreatePool(amount, token, starting_price) => {
                AMM::create_pool(db, sender, *amount, *token, *starting_price)
            }
            Action::AddLiquidity(amount, token) => AMM::add_liquidity(db, sender, *amount, *token),
            Action::RemoveLiquidity(percentage, token) => {
                AMM::remove_liquidity(db, sender, *percentage, *token)
            }
            Action::Trade(input_amount, input_token, minimum_output_token_amount, output_token) => {
                AMM::trade(
                    db,
                    sender,
                    *input_amount,
                    *input_token,
                    *minimum_output_token_amount,
                    *output_token,
                )
            }
            Action::Harvest() => Ellipticoin::harvest(db, sender),
            Action::Vote(proposal_id, vote) => {
                Governance::vote(db, sender, *proposal_id, vote.clone())
            }
            Action::CreateRedeemRequest(amount, token) => {
                Bridge::create_redeem_request(db, sender, *amount, *token)
            }
            _ => bail!("Unknown transaction type"),
        };
        if result.is_ok() {
            db.commit();
        } else {
            db.revert();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::System;
    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB},
            tokens::APPLES,
        },
        TestDB,
    };

    #[test]
    fn test_run() {
        let mut db = TestDB::new();
        Token::set_balance(&mut db, ALICE, APPLES, 100);
        let transfer_transaction = Transaction {
            network_id: 0,
            transaction_number: 0,
            action: Action::Pay(BOB, 20, APPLES),
        };
        transfer_transaction.run(&mut db, ALICE).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 80);
        assert_eq!(Token::get_balance(&mut db, BOB, APPLES), 20);
        assert_eq!(System::get_transaction_number(&mut db, ALICE), 1);
    }
}
