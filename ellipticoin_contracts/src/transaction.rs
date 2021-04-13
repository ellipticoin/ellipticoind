use crate::{
    bridge::Update, constants::LEVERAGED_BASE_TOKEN, governance::Choice, order_book::OrderType,
    Bridge, Ellipticoin, Governance, OrderBook, System, Token, AMM,
};
use anyhow::Result;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub transaction_number: u64,
    pub network_id: u64,
    pub action: Action,
}

impl Transaction {
    pub fn run<B: Backend>(&self, db: &mut Db<B>, sender: Address) -> Result<()> {
        self.action.run(db, sender)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Action {
    AddLiquidity(u64, Address),
    CreateOrder(OrderType, u64, Address, u64),
    CreatePool(u64, Address, u64),
    CreateProposal(String, String, String, Vec<Action>),
    CreateRedeemRequest(u64, Address),
    FillOrder(u64, OrderType, u64, Address, u64),
    Harvest(),
    Migrate([u8; 32], Vec<u8>),
    Pay(Address, u64, Address),
    RemoveLiquidity(u64, Address),
    Seal([u8; 32]),
    SignRedeemRequest(u64, u64, Vec<u8>),
    StartBridge(u64),
    StartMining(String, [u8; 32], u64),
    Trade(u64, Address, u64, Address),
    Update(Update),
    Vote(usize, Choice),
}
impl Action {
    pub fn run<B: Backend>(&self, db: &mut Db<B>, sender: Address) -> Result<()> {
        System::increment_transaction_number(db, sender);
        let result = match &self {
            Action::AddLiquidity(amount, token) => AMM::add_liquidity(db, sender, *amount, *token),
            Action::CreateOrder(order_type, underlying_amount, token, underlying_price) => {
                let amount = Token::underlying_to_amount(db, *underlying_amount, *token);
                let price = Token::amount_to_underlying(db, *underlying_price, *token);
                OrderBook::create_order(db, sender, order_type.clone(), amount, *token, price)
            }
            Action::CreatePool(amount, token, underlying_starting_price) => {
                let starting_price = Token::underlying_to_amount(
                    db,
                    *underlying_starting_price,
                    LEVERAGED_BASE_TOKEN,
                );
                AMM::create_pool(db, sender, *amount, *token, starting_price)
            }
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
            Action::CreateRedeemRequest(amount, token) => {
                Bridge::create_redeem_request(db, sender, *amount, *token)
            }
            Action::FillOrder(order_id, _order_type, _amount, _token, _price) => {
                OrderBook::fill(db, sender, *order_id)
            }
            Action::Harvest() => Ellipticoin::harvest(db, sender),
            Action::Migrate(legacy_address, legacy_signature) => {
                System::migrate(db, sender, *legacy_address, legacy_signature.to_vec())
            }
            Action::Pay(recipient, underlying_amount, token) => {
                let amount = Token::underlying_to_amount(db, *underlying_amount, *token);
                Token::transfer(db, sender, *recipient, amount, *token)
            }
            Action::RemoveLiquidity(percentage, token) => {
                AMM::remove_liquidity(db, sender, *percentage, *token)
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
            Action::StartBridge(ethereum_block_number) => Bridge::start(db, *ethereum_block_number),
            Action::StartMining(host, onion_skin, layer_count) => {
                Ellipticoin::start_mining(db, sender, host.to_string(), *onion_skin, *layer_count)
            }
            Action::Trade(
                underlying_input_amount,
                input_token,
                minimum_underlying_output_amount,
                output_token,
            ) => {
                let input_amount =
                    Token::underlying_to_amount(db, *underlying_input_amount, *input_token);
                let minimum_output_amount = Token::underlying_to_amount(
                    db,
                    *minimum_underlying_output_amount,
                    *output_token,
                );

                AMM::trade(
                    db,
                    sender,
                    input_amount,
                    *input_token,
                    minimum_output_amount,
                    *output_token,
                )
            }
            Action::Update(update) => Bridge::update(db, update.clone()),
            Action::Vote(proposal_id, vote) => {
                Governance::vote(db, sender.clone(), *proposal_id, vote.clone())
            }
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
        new_db,
    };

    #[test]
    fn test_run() {
        let mut db = new_db();
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
