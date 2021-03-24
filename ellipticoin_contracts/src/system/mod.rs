use anyhow::{Result};
use crate::{
    constants::TOKENS,
    crypto::ed25519_verify,
    Ellipticoin,
    Token,
};
use std::convert::TryInto;
use crate::AMM;
use crate::contract::{self, Contract};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};


pub struct System;

impl Contract for System {
    const NAME: contract::Name = contract::Name::System;
}

db_accessors!(System {
    block_number() -> u64;
    transaction_number(address: Address) -> u64;
});

impl System {
    pub fn get_next_transaction_number<B: Backend>(db: &mut Db<B>, address: Address) -> u64 {
        if Self::get_transaction_number(db, address) == 0 {
            1
        } else {
            Self::get_transaction_number(db, address) + 1
        }
    }
    pub fn increment_block_number<B: Backend>(db: &mut Db<B>) {
        let block_number = Self::get_block_number(db) + 1;
        Self::set_block_number(db, block_number);
    }

    pub fn increment_transaction_number<B: Backend>(db: &mut Db<B>, address: Address) {
        let transaction_number = System::get_next_transaction_number(db, address);
        Self::set_transaction_number(db, address, transaction_number);
    }

    pub fn migrate<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        legacy_address: [u8; 32],
        legacy_signature: Vec<u8>,
    ) -> Result<()> {
        ed25519_verify(sender.as_ref(), &legacy_address, &legacy_signature)?;
        Ellipticoin::harvest(db, Address(legacy_address[..20].try_into().unwrap()))?;
        for token in [
            TOKENS.to_vec(),
        ]
        .concat()
        .iter()
        {
            let balance = Token::get_balance(db, Address(legacy_address[..20].try_into().unwrap()), *token);
            Token::transfer(
                db,
                Address(legacy_address[..20].try_into().unwrap()),
                sender,
                balance,
                *token,
            )?;
        }

        for token in TOKENS.iter() {
            let legacy_address: Address = Address(legacy_address[..20].try_into().unwrap());
            if AMM::get_liquidity_providers(db, *token).contains(&legacy_address) {
                let balance = AMM::get_balance(db, legacy_address, *token);
                println!("{} {} {}", hex::encode(legacy_address), hex::encode(token), balance);
                AMM::transfer(
                    db,
                    legacy_address,
                    sender,
                    balance,
                    *token,
                )?;
                let mut liquidity_providers = AMM::get_liquidity_providers(db, *token);
                liquidity_providers.remove(&legacy_address);
                liquidity_providers.insert(sender);
                AMM::set_liquidity_providers(db, *token, liquidity_providers);
            }
        }

        Ok(())
    }
}
