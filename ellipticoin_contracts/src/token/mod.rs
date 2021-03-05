pub mod macros;

use crate::{
    constants::TOKENS,
    contract::{self, Contract},
    crypto::ed25519_verify,
    Ellipticoin, AMM,
};
use anyhow::{bail, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::Address;
use std::convert::TryInto;

use hex;

pub struct Token;

impl Contract for Token {
    const NAME: contract::Name = contract::Name::Token;
}

db_accessors!(Token {
    balance(address: Address, token: Address) -> u64;
    total_supply(token: Address) -> u64;
});

impl Token {
    pub fn migrate<D: ellipticoin_types::DB>(
        db: &mut D,
        sender: Address,
        legacy_address: [u8; 32],
        legacy_signature: Vec<u8>,
    ) -> Result<()> {
        ed25519_verify(&sender, &legacy_address, &legacy_signature)?;
        Ellipticoin::harvest(db, legacy_address[..20].try_into().unwrap())?;
        for token in [
            TOKENS.to_vec(),
            TOKENS
                .iter()
                .map(|token| AMM::liquidity_token(*token))
                .collect::<Vec<Address>>(),
        ]
        .concat()
        .iter()
        {
            let balance = Self::get_balance(db, legacy_address[..20].try_into().unwrap(), *token);
            Self::transfer(
                db,
                legacy_address[..20].try_into().unwrap(),
                sender,
                balance,
                *token,
            )?;
        }
        for token in TOKENS.iter() {
            let legacy_address: [u8; 20] = legacy_address[..20].try_into().unwrap();
            if AMM::get_liquidity_providers(db, *token).contains(&legacy_address) {
                let mut liquidity_providers = AMM::get_liquidity_providers(db, *token);
                liquidity_providers.insert(sender);
                AMM::set_liquidity_providers(db, *token, liquidity_providers);
            }
        }

        Ok(())
    }

    pub fn transfer<D: ellipticoin_types::DB>(
        db: &mut D,
        sender: Address,
        recipient: Address,
        amount: u64,
        token: Address,
    ) -> Result<()> {
        Self::debit(db, amount, token, sender)?;
        Self::credit(db, amount, token, recipient);
        Ok(())
    }

    pub fn mint<D: ellipticoin_types::DB>(
        db: &mut D,
        amount: u64,
        token: Address,
        address: Address,
    ) {
        Self::credit(db, amount, token, address);
        let total_supply = Self::get_total_supply(db, token);
        Self::set_total_supply(db, token, total_supply + amount);
    }

    pub fn burn<D: ellipticoin_types::DB>(
        db: &mut D,
        amount: u64,
        token: Address,
        address: Address,
    ) -> Result<()> {
        Self::debit(db, amount, token, address)?;
        let total_supply = Self::get_total_supply(db, token);
        Self::set_total_supply(db, token, total_supply - amount);
        Ok(())
    }

    pub fn credit<D: ellipticoin_types::DB>(
        db: &mut D,
        amount: u64,
        token: Address,
        address: Address,
    ) {
        let balance = Self::get_balance(db, address, token);
        Self::set_balance(db, address, token, balance + amount)
    }

    fn debit<D: ellipticoin_types::DB>(
        db: &mut D,
        amount: u64,
        token: Address,
        address: Address,
    ) -> Result<()> {
        let balance = Self::get_balance(db, address, token);

        if amount <= balance {
            Ok(Self::set_balance(db, address, token, balance - amount))
        } else {
            bail!(
                "{} has insufficient balance of {} have {} need {}",
                hex::encode(address),
                hex::encode(token),
                balance,
                amount
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Token;
    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB},
            tokens::APPLES,
        },
        test_db::TestDB,
    };

    #[test]
    fn test_transfer() {
        let mut db = TestDB::new();
        Token::set_balance(&mut db, ALICE, APPLES, 100);
        Token::transfer(&mut db, ALICE, BOB, 20, APPLES).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 80);
        assert_eq!(Token::get_balance(&mut db, BOB, APPLES), 20);
    }
}
