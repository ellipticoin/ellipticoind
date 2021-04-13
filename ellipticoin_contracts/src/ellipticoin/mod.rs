mod issuance;

use crate::{
    constants::{INCENTIVISED_POOLS, MINER_ALLOW_LIST},
    contract::{self, Contract},
    crypto::sha256,
    pay, System, Token, AMM,
};
use anyhow::{anyhow, bail, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

pub struct Ellipticoin;

impl Contract for Ellipticoin {
    const NAME: contract::Name = contract::Name::Ellipticoin;
}

db_accessors!(Ellipticoin {
    issuance_rewards(address: Address) -> u64;
    miners() -> Vec<Miner>;
});
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Miner {
    pub host: String,
    pub address: Address,
    pub hash_onion_skin: [u8; 32],
    pub hash_onion_layers_left: u64,
}

impl Ellipticoin {
    pub fn start_mining<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        host: String,
        hash_onion_skin: [u8; 32],
        layer_count: u64,
    ) -> Result<()> {
        let mut miners = Self::get_miners(db);
        if !MINER_ALLOW_LIST.contains(&sender) {
            bail!("Miner {} is not on the allow list", hex::encode(sender));
        }
        miners.push(Miner {
            address: sender,
            host,
            hash_onion_skin,
            hash_onion_layers_left: layer_count,
        });
        Self::set_miners(db, miners);
        Ok(())
    }

    pub fn seal<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        hash_onion_skin: [u8; 32],
    ) -> Result<()> {
        let mut miners = Self::get_miners(db);
        if sender
            != miners
                .first()
                .ok_or_else(|| anyhow!("No miners running"))?
                .address
        {
            bail!(
                "Winning miner was {} but sender was {}",
                hex::encode(miners.first().unwrap().address),
                hex::encode(sender)
            )
        };
        if !miners
            .first()
            .unwrap()
            .hash_onion_skin
            .to_vec()
            .eq(&sha256(hash_onion_skin.to_vec()))
        {
            bail!(
                "Invalid onion skin: expected {} but got {}",
                base64::encode(&miners.first().unwrap().hash_onion_skin),
                base64::encode(&sha256(hash_onion_skin.to_vec()))
            );
        }
        miners.first_mut().unwrap().hash_onion_skin = hash_onion_skin.clone();
        miners.first_mut().unwrap().hash_onion_layers_left -= 1;
        Self::settle_block_rewards(db)?;
        Self::shuffle_miners(db, &mut miners, hash_onion_skin);
        Self::issue_block_rewards(db)?;
        System::increment_block_number(db);

        Ok(())
    }

    pub fn harvest<B: Backend>(db: &mut Db<B>, sender: Address) -> Result<()> {
        let issuance_rewards = Self::get_issuance_rewards(db, sender);
        Self::debit_issuance_rewards(db, sender, issuance_rewards);
        pay!(db, sender, Self::address(), issuance_rewards)?;
        Ok(())
    }

    fn issue_block_rewards<B: Backend>(db: &mut Db<B>) -> Result<()> {
        let block_number = System::get_block_number(db);
        let block_reward = Self::block_reward_at(block_number);
        Self::mint(db, block_reward);
        let reward_per_pool = block_reward / INCENTIVISED_POOLS.len() as u64;
        for token in INCENTIVISED_POOLS.iter() {
            let liquidity_providers = AMM::get_liquidity_providers(db, token.clone());
            let (addresses, balances): (Vec<Address>, Vec<u64>) = liquidity_providers
                .iter()
                .map(|address| (address, AMM::get_balance(db, *address, *token)))
                .unzip();

            addresses
                .iter()
                .zip(distribute(reward_per_pool, balances).iter())
                .for_each(|(address, issuance)| {
                    Self::credit_issuance_rewards(db, address.clone(), *issuance);
                });
        }
        Ok(())
    }

    fn shuffle_miners<B: Backend>(db: &mut Db<B>, miners: &mut Vec<Miner>, value: [u8; 32]) {
        let mut rng = StdRng::from_seed(value[0..32].try_into().unwrap());
        let mut shuffled_miners = vec![];
        while !miners.is_empty() {
            let random_miner = miners.choose(&mut rng).unwrap().clone();
            shuffled_miners.push(random_miner.clone());
            miners.retain(|miner| miner.clone() != random_miner);
        }
        *miners = shuffled_miners.clone();
        Self::set_miners(db, shuffled_miners);
    }

    fn settle_block_rewards<B: Backend>(db: &mut Db<B>) -> Result<()> {
        let miners = Self::get_miners(db);
        let winner = miners.first().as_ref().unwrap().clone();
        for miner in &miners {
            Self::transfer(db, miner.address.clone(), winner.address.clone(), 0)?;
        }
        Ok(())
    }

    fn transfer<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        recipient: Address,
        amount: u64,
    ) -> Result<()> {
        Token::transfer(db, sender, recipient, amount, Self::address())
    }

    fn mint<B: Backend>(db: &mut Db<B>, amount: u64) {
        Token::credit(db, amount, Self::address(), Self::address())
    }

    fn credit_issuance_rewards<B: Backend>(db: &mut Db<B>, address: Address, amount: u64) {
        let issuance_rewards = Self::get_issuance_rewards(db, address.clone());
        Self::set_issuance_rewards(db, address, issuance_rewards + amount);
    }

    fn debit_issuance_rewards<B: Backend>(db: &mut Db<B>, address: Address, amount: u64) {
        let issuance_rewards = Self::get_issuance_rewards(db, address.clone());
        Self::set_issuance_rewards(db, address, issuance_rewards - amount);
    }
}

fn distribute(mut amount: u64, mut values: Vec<u64>) -> Vec<u64> {
    let mut rest = values.clone();
    let mut distributions: Vec<u64> = Default::default();
    values.reverse();
    for balance in values.clone() {
        let denominator = rest.iter().sum::<u64>();
        let distribution = if denominator == 0 {
            0
        } else {
            (amount * balance) / denominator
        };
        amount -= distribution;
        distributions.push(distribution);
        rest.pop();
    }
    distributions.reverse();
    distributions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_onion;
    use ellipticoin_test_framework::{
        constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB, BOBS_PRIVATE_KEY},
        new_db, setup,
    };

    #[test]
    fn test_commit_and_seal() {
        let elc: Address = Ellipticoin::address();
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![(5, elc)],
                BOB => vec![(5, elc)]
            },
        );
        let mut alices_onion = hash_onion::generate(3, ALICES_PRIVATE_KEY);
        let mut bobs_onion = hash_onion::generate(3, BOBS_PRIVATE_KEY);
        Ellipticoin::set_miners(
            &mut db,
            vec![
                Miner {
                    address: ALICE,
                    host: "host1".to_string(),
                    hash_onion_skin: *alices_onion.last().unwrap(),
                    hash_onion_layers_left: alices_onion.len() as u64,
                },
                Miner {
                    address: BOB,
                    host: "host2".to_string(),
                    hash_onion_skin: *bobs_onion.last().unwrap(),
                    hash_onion_layers_left: bobs_onion.len() as u64,
                },
            ],
        );
        alices_onion.pop();
        assert!(Ellipticoin::seal(&mut db, ALICE, *alices_onion.last().unwrap()).is_ok());
        bobs_onion.pop();
        assert!(Ellipticoin::seal(&mut db, BOB, *bobs_onion.last().unwrap()).is_ok());

        alices_onion.pop();
        assert!(Ellipticoin::seal(&mut db, ALICE, *alices_onion.last().unwrap()).is_ok());
        assert_eq!(Token::get_balance(&mut db, ALICE, elc), 5);
        assert_eq!(System::get_block_number(&mut db), 3);
    }
}
