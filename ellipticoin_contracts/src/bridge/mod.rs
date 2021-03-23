use crate::{
    charge,
    contract::{self, Contract},
    pay,
    token::Token,
};
use anyhow::{anyhow, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Mint(pub u64, pub Address, pub Address);
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Redeem(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Update {
    pub block_number: u64,
    pub base_token_exchange_rate: BigInt,
    pub base_token_interest_rate: u64,
    pub mints: Vec<Mint>,
    pub redeems: Vec<Redeem>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RedeemRequest {
    pub id: u64,
    pub sender: Address,
    pub token: Address,
    pub amount: u64,
    pub expiration_block_number: Option<u64>,
    pub signature: Option<Vec<u8>>,
}

pub struct Bridge;

impl Contract for Bridge {
    const NAME: contract::Name = contract::Name::Bridge;
}

db_accessors!(Bridge {
    ethereum_block_number() -> u64;
    pending_redeem_requests() -> Vec<RedeemRequest>;
    redeem_id_counter() -> u64;
    signature(transaction_id: u64) -> Vec<u8>;
});

impl Bridge {
    pub fn start<B: Backend>(db: &mut Db<B>, ethereum_block_number: u64) -> Result<()> {
        Bridge::set_ethereum_block_number(db, ethereum_block_number);
        Ok(())
    }
    pub fn update<B: Backend>(db: &mut Db<B>, update: Update) -> Result<()> {
        match update {
            Update {
                block_number,
                base_token_exchange_rate,
                base_token_interest_rate,
                mints,
                redeems,
            } => {
                Token::set_base_token_exchange_rate(db, base_token_exchange_rate);
                Token::set_base_token_interest_rate(db, base_token_interest_rate);
                let pending_redeem_requests = Bridge::get_pending_redeem_requests(db);
                for pending_redeem_request in pending_redeem_requests.iter() {
                    if block_number > pending_redeem_request.expiration_block_number.unwrap() {
                        Bridge::cancel_redeem_request(db, pending_redeem_request.id).unwrap();
                    }
                }
                for Mint(amount, token, address) in mints.iter() {
                    Bridge::mint(db, *amount, *token, *address).unwrap();
                }
                for Redeem(redeem_id) in redeems.iter() {
                    Bridge::redeem(db, *redeem_id).unwrap();
                }
                Bridge::set_ethereum_block_number(db, block_number);
                Ok(())
            }
        }
    }

    pub fn mint<B: Backend>(
        db: &mut Db<B>,
        amount: u64,
        token: Address,
        address: Address,
    ) -> Result<()> {
        Token::mint(db, amount, token, address);
        Ok(())
    }

    pub fn create_redeem_request<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        amount: u64,
        token: Address,
    ) -> Result<()> {
        charge!(db, sender, token, amount)?;
        let mut pending_redeem_requests = Self::get_pending_redeem_requests(db);
        pending_redeem_requests.push(RedeemRequest {
            id: Self::get_redeem_id_counter(db),
            sender,
            amount,
            token,
            expiration_block_number: None,
            signature: None,
        });
        Self::increment_redeem_id_counter(db);
        Self::set_pending_redeem_requests(db, pending_redeem_requests);
        Ok(())
    }

    pub fn sign_redeem_request<B: Backend>(
        db: &mut Db<B>,
        redeem_id: u64,
        expiration_block_number: u64,
        signature: Vec<u8>,
    ) -> Result<()> {
        let mut pending_redeem_requests = Self::get_pending_redeem_requests(db);
        let index = pending_redeem_requests
            .iter()
            .cloned()
            .position(|pending_redeem_request| pending_redeem_request.id == redeem_id)
            .ok_or(anyhow!("Redeem request {} not found", redeem_id))?;
        pending_redeem_requests[index].expiration_block_number = Some(expiration_block_number);
        pending_redeem_requests[index].signature = Some(signature);
        Self::set_pending_redeem_requests(db, pending_redeem_requests);
        Ok(())
    }

    pub fn cancel_redeem_request<B: Backend>(db: &mut Db<B>, redeem_id: u64) -> Result<()> {
        let pending_redeem_request = Self::remove_redeem_request(db, redeem_id)?;
        pay!(
            db,
            pending_redeem_request.sender,
            pending_redeem_request.token,
            pending_redeem_request.amount
        )?;
        Ok(())
    }

    pub fn redeem<B: Backend>(db: &mut Db<B>, redeem_id: u64) -> Result<()> {
        let pending_redeem_request = Self::remove_redeem_request(db, redeem_id)?;
        Token::burn(
            db,
            pending_redeem_request.amount,
            pending_redeem_request.token,
            Self::address(),
        )?;
        Ok(())
    }

    fn increment_redeem_id_counter<B: Backend>(db: &mut Db<B>) -> u64 {
        let redeem_id_counter = Self::get_redeem_id_counter(db) + 1;
        Self::set_redeem_id_counter(db, redeem_id_counter);
        redeem_id_counter
    }

    pub fn remove_redeem_request<B: Backend>(
        db: &mut Db<B>,
        redeem_id: u64,
    ) -> Result<RedeemRequest> {
        let mut pending_redeem_requests = Self::get_pending_redeem_requests(db);
        let index = pending_redeem_requests
            .iter()
            .cloned()
            .position(|pending_redeem_request| pending_redeem_request.id == redeem_id)
            .ok_or(anyhow!("Redeem request {} not found", redeem_id))?;
        let pending_redeem_request = pending_redeem_requests[index].clone();
        pending_redeem_requests.remove(index);
        Self::set_pending_redeem_requests(db, pending_redeem_requests);
        Ok(pending_redeem_request)
    }
}

#[cfg(test)]
mod tests {
    use super::Bridge;
    use crate::{constants::BASE_FACTOR, Token};
    use ellipticoin_test_framework::{
        constants::{actors::ALICE, tokens::APPLES},
        new_db,
    };

    #[test]
    fn test_mint() {
        let mut db = new_db();
        Bridge::mint(&mut db, 1 * BASE_FACTOR, APPLES, ALICE).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES,), 1 * BASE_FACTOR);
    }

    #[test]
    fn test_redeem() {
        let mut db = new_db();
        Bridge::mint(&mut db, 1 * BASE_FACTOR, APPLES, ALICE).unwrap();
        Bridge::create_redeem_request(&mut db, ALICE, 1 * BASE_FACTOR, APPLES).unwrap();
        Bridge::redeem(&mut db, 0).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 0);
    }

    #[test]
    fn test_create_redeem_request() {
        let mut db = new_db();
        Bridge::mint(&mut db, 1 * BASE_FACTOR, APPLES, ALICE).unwrap();
        Bridge::create_redeem_request(&mut db, ALICE, 1 * BASE_FACTOR, APPLES).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 0 * BASE_FACTOR);
    }

    #[test]
    fn test_sign_redeem_request() {
        let mut db = new_db();
        Bridge::mint(&mut db, 1 * BASE_FACTOR, APPLES, ALICE).unwrap();
        Bridge::create_redeem_request(&mut db, ALICE, 1 * BASE_FACTOR, APPLES).unwrap();
        Bridge::sign_redeem_request(&mut db, 0, 1, vec![1, 2, 3]).unwrap();
        assert_eq!(
            Bridge::get_pending_redeem_requests(&mut db)
                .first()
                .unwrap()
                .signature
                .as_ref()
                .unwrap()
                .to_vec(),
            vec![1, 2, 3]
        );
        assert_eq!(
            Bridge::get_pending_redeem_requests(&mut db)
                .first()
                .unwrap()
                .expiration_block_number
                .as_ref()
                .unwrap()
                .clone(),
            1
        );
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 0 * BASE_FACTOR);
    }

    #[test]
    fn test_cancel_redeem_request() {
        let mut db = new_db();
        Bridge::mint(&mut db, 1 * BASE_FACTOR, APPLES, ALICE).unwrap();
        Bridge::create_redeem_request(&mut db, ALICE, 1 * BASE_FACTOR, APPLES).unwrap();
        Bridge::cancel_redeem_request(&mut db, 0).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 1 * BASE_FACTOR);
    }
}
