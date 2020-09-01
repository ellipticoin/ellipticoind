mod errors;
mod ethereum;
mod hashing;

use super::token;
use ellipticoin::{constants::ELC, storage_accessors, Address};
use errors::Error;
use hashing::sha256;
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::convert::TryInto;
use wasm_rpc_macros::export_native;

const CONTRACT_NAME: &'static str = "Ellipticoin";

storage_accessors!(
    ethereum_balances(address: Vec<u8>) -> u64;
    miners() -> Vec<Miner>;
    unlocked_ethereum_balances(ethereum_address: Vec<u8>) -> bool;
    total_unlocked_ethereum() -> u64;
);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Miner {
    pub host: String,
    pub address: [u8; 32],
    pub burn_per_block: u64,
    pub hash_onion_skin: [u8; 32],
}

export_native! {
    pub fn transfer_to_current_miner<API: ellipticoin::API>(api: &mut API, amount: u64) -> Result<(), Box<Error>> {
        let miners = get_miners(api);
        let current_miner = miners.first().unwrap().address.clone();
        token::transfer_from(api, ELC.clone(), api.caller(), ellipticoin::Address::PublicKey(current_miner), amount)?;
    Ok(())
    }

    pub fn unlock_ether<API: ellipticoin::API>(
        api: &mut API,
        unlock_signature: [u8; 32],
        ellipticoin_address: [u8; 32],
    ) -> Result<Value, Box<Error>> {
        let encoded_ellipticoin_adress =
            base64::encode_config(&ellipticoin_address, base64::URL_SAFE_NO_PAD);
        let message = format!(
            "Unlock Ellipticoin at address: {}",
            encoded_ellipticoin_adress
        );
        let address = ethereum::ecrecover_address(message.as_bytes(), &unlock_signature);
        if get_unlocked_ethereum_balances(api, address.clone()) {
            return Err(Box::new(errors::BALANCE_ALREADY_UNLOCKED.clone()));
        };
        let balance: u64 = get_ethereum_balances(api, address.clone());
        let mut total_unlocked_ethereum: u64 = get_total_unlocked_ethereum(api);
        if total_unlocked_ethereum + balance > 1000000 * 10000 {
            return Err(Box::new(errors::BALANCE_EXCEEDS_THIS_PHASE.clone()));
        } else {
            total_unlocked_ethereum += balance;
            set_total_unlocked_ethereum(api, total_unlocked_ethereum);
        }
        native::credit(api, Address::PublicKey(ellipticoin_address), balance);
        native::set_unlocked_ethereum_balances(api, address, true);

        Ok(balance.into())
    }

    pub fn start_mining<API: ellipticoin::API>(
        api: &mut API,
        host: String,
        burn_per_block: u64,
        hash_onion_skin: [u8; 32],
    ) -> Result<Value, Box<Error>> {
        let mut miners = get_miners(api);
        miners.push(Miner {
            address: api.caller().as_public_key().unwrap(),
            host,
            burn_per_block,
            hash_onion_skin,
        });
        set_miners(api, miners);
        Ok(Value::Null)
    }

    pub fn reveal<API: ellipticoin::API>(api: &mut API, value: [u8; 32]) -> Result<Vec<Miner>, Box<Error>> {
        let mut miners = get_miners(api);
        if api.caller() != ellipticoin::Address::PublicKey(miners.first().unwrap().address) {
            return Err(Box::new(errors::SENDER_IS_NOT_THE_WINNER.clone()));
        }
        if !miners
            .first()
            .unwrap()
            .hash_onion_skin
            .to_vec()
            .eq(&sha256(value.to_vec()))
        {
            return Err(Box::new(errors::INVALID_VALUE.clone()));
        }
        settle_block_rewards(api)?;
        miners.first_mut().unwrap().hash_onion_skin = value.clone();
        shuffle_miners(api, &mut miners, value);

        Ok(miners)
    }

    fn shuffle_miners<API: ellipticoin::API>(api: &mut API, miners: &mut Vec<Miner>, value: [u8; 32]) {
        let mut rng = SmallRng::from_seed(value[0..16].try_into().unwrap());
        let mut shuffled_miners = vec![];
        while !miners.is_empty() {
            let random_miner = miners
                .choose_weighted(&mut rng, |miner| miner.burn_per_block)
                .unwrap()
                .clone();
            shuffled_miners.push(random_miner.clone());
            miners.retain(|miner| miner.clone() != random_miner);
        }
        *miners = shuffled_miners.clone();
        set_miners(api, shuffled_miners);
    }

    fn settle_block_rewards<API: ellipticoin::API>(api: &mut API) -> Result<(), Box<Error>> {
        let miners = get_miners(api);
        let winner = miners.first().as_ref().unwrap().clone();
        for miner in &miners {
            credit(api, ellipticoin::Address::PublicKey(winner.address.clone()), miner.burn_per_block);
            debit(api, ellipticoin::Address::PublicKey(miner.address.clone()), miner.burn_per_block)?;
        }
        Ok(())
    }

}

fn credit<API: ellipticoin::API>(api: &mut API, address: Address, amount: u64) {
    token::credit(api, ELC.clone(), address, amount);
}

fn debit<API: ellipticoin::API>(
    api: &mut API,
    address: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    token::debit(api, ELC.clone(), address, amount)?;
    Ok(())
}

fn _distribute(mut amount: u64, mut values: Vec<u64>) -> Vec<u64> {
    let mut rest = values.clone();
    let mut distributions: Vec<u64> = Default::default();
    values.reverse();
    for balance in values.clone() {
        let distribution = (amount * balance) / rest.iter().sum::<u64>();
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
    use crate::{
        config::HOST,
        helpers::generate_hash_onion,
        system_contracts::{
            test_api::{TestAPI, TestState},
            token::get_balance,
        },
    };
    use ellipticoin::constants::SYSTEM_ADDRESS;
    use std::env;

    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB};

    #[test]
    fn test_commit_and_reveal() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        env::set_var("HOST", "localhost");
        let mut state = TestState::new();
        let mut api = TestAPI::new(
            &mut state,
            *ALICE,
            (SYSTEM_ADDRESS, "Ellipticoin".to_string()),
        );
        credit(&mut api, Address::PublicKey(*ALICE), 5);
        credit(&mut api, Address::PublicKey(*BOB), 5);
        let alices_center = [0; 32];
        let bobs_center = [1; 32];
        let mut alices_onion = generate_hash_onion(3, alices_center.clone());
        let mut bobs_onion = generate_hash_onion(3, bobs_center.clone());
        api.caller = Address::PublicKey(*ALICE);
        native::start_mining(&mut api, HOST.to_string(), 1, *alices_onion.last().unwrap()).unwrap();
        api.caller = Address::PublicKey(*BOB);
        native::start_mining(&mut api, HOST.to_string(), 1, *bobs_onion.last().unwrap()).unwrap();

        // With this random seed the winners are Alice, Alice, Bob in that order
        api.caller = Address::PublicKey(*ALICE);
        alices_onion.pop();
        assert!(native::reveal(&mut api, *alices_onion.last().unwrap()).is_ok());
        alices_onion.pop();
        assert!(native::reveal(&mut api, *alices_onion.last().unwrap()).is_ok());
        api.caller = Address::PublicKey(*BOB);
        bobs_onion.pop();
        assert!(native::reveal(&mut api, *bobs_onion.last().unwrap()).is_ok());
        assert_eq!(
            get_balance(&mut api, ELC.clone(), Address::PublicKey(*ALICE)),
            6
        );
        assert_eq!(
            get_balance(&mut api, ELC.clone(), Address::PublicKey(*BOB)),
            4
        );
    }
}
