mod errors;
mod ethereum;
mod hashing;
mod issuance;

use super::token;
use crate::system_contracts::{
    ellipticoin::issuance::INCENTIVISED_POOLS, exchange, exchange::pool_token, token::mint,
};
use ellipticoin::{constants::ELC, memory_accessors, pay, storage_accessors, Address, PublicKey};

use errors::Error;
use hashing::sha256;
use issuance::block_reward_at;
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::convert::TryInto;
use std::collections::HashSet;
use wasm_rpc_macros::export_native;

const CONTRACT_NAME: &'static str = "Ellipticoin";

lazy_static! {
    pub static ref ADDRESS: std::string::String = CONTRACT_NAME.to_string();
}

storage_accessors!(
    block_number() -> u32;
    ethereum_balances(address: Vec<u8>) -> u64;
    miners() -> Vec<Miner>;
    total_unlocked_ethereum() -> u64;
    unlocked_ethereum_balances(ethereum_address: Vec<u8>) -> bool;
    miner_whitelist() -> HashSet<[u8; 32]>;
);

memory_accessors!(
    issuance_rewards(address: Address) -> u64;
);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub struct Miner {
    pub host: String,
    pub address: PublicKey,
    pub burn_per_block: u64,
    pub hash_onion_skin: [u8; 32],
}

export_native! {
    pub fn harvest<API: ellipticoin::API>(api: &mut API) {
        let issuance_rewards = get_issuance_rewards(api, api.caller());
        debit_issuance_rewards(api, api.caller(), issuance_rewards);
        pay!(api, ELC.clone(), api.caller(), issuance_rewards).unwrap();
    }

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

    pub fn whitelist_miner<API: ellipticoin::API>(
        api: &mut API,
        address: [u8; 32],
    ) -> Result<Value, Box<Error>> {
        let mut whitelist = get_miner_whitelist(api);
        let caller_address = api.caller().as_public_key().unwrap();
        if whitelist.is_empty() {
            whitelist.insert(caller_address);
        } else {
            if !whitelist.contains(&caller_address) {
                return Err(Box::new(errors::MINER_IS_NOT_WHITELISTED.clone()));
            }
        }

        whitelist.insert(address);
        set_miner_whitelist(api, whitelist);
        return Ok(Value::Null)
    }

    pub fn start_mining<API: ellipticoin::API>(
        api: &mut API,
        host: String,
        burn_per_block: u64,
        hash_onion_skin: [u8; 32],
    ) -> Result<Value, Box<Error>> {
        let mut miners = get_miners(api);
        let whitelist = get_miner_whitelist(api);
        let address = api.caller().as_public_key().unwrap();
        if !whitelist.is_empty() && !whitelist.contains(&address) {
            return Err(Box::new(errors::MINER_IS_NOT_WHITELISTED.clone()));
        }
        miners.push(Miner {
            address,
            host,
            burn_per_block,
            hash_onion_skin,
        });
        set_miners(api, miners);
        Ok(Value::Null)
    }

    pub fn seal<API: ellipticoin::API>(api: &mut API, value: [u8; 32]) -> Result<Vec<Miner>, Box<Error>> {
        let mut miners = get_miners(api);
        if api.caller() != ellipticoin::Address::PublicKey(miners.first().unwrap().address) {
            println!("sender is not the winner");
            return Err(Box::new(errors::SENDER_IS_NOT_THE_WINNER.clone()));
        }
        if !miners
            .first()
            .unwrap()
            .hash_onion_skin
            .to_vec()
            .eq(&sha256(value.to_vec()))
        {
            println!("invalid value submitted");
            return Err(Box::new(errors::INVALID_VALUE.clone()));
        }
        miners.first_mut().unwrap().hash_onion_skin = value.clone();
        settle_block_rewards(api)?;
        shuffle_miners(api, &mut miners, value);
        issue_block_rewards(api)?;
        increment_block_number(api);

        Ok(miners)
    }


}
fn issue_block_rewards<API: ellipticoin::API>(api: &mut API) -> Result<(), Box<Error>> {
    let block_reward = block_reward_at(get_block_number(api));
    mint(
        api,
        ELC.clone(),
        Address::Contract(ADDRESS.clone()),
        block_reward,
    )?;
    let reward_per_pool = block_reward / INCENTIVISED_POOLS.len() as u64;
    for token in INCENTIVISED_POOLS.clone() {
        let share_holders = exchange::get_share_holders(api, token.clone());
        let (addresses, balances): (Vec<_>, Vec<_>) = share_holders
            .iter()
            .cloned()
            .map(|address| {
                (
                    address.clone(),
                    token::get_balance(api, pool_token(token.clone()), address),
                )
            })
            .collect::<Vec<(Address, u64)>>()
            .iter()
            .cloned()
            .unzip();

        for (address, issuance) in addresses
            .iter()
            .zip(distribute(reward_per_pool, balances).iter())
        {
            credit_issuance_rewards(api, address.clone(), *issuance);
        }
    }
    Ok(())
}

fn increment_block_number<API: ellipticoin::API>(api: &mut API) {
    let block_number = get_block_number(api);
    set_block_number(api, block_number + 1);
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
        credit(
            api,
            ellipticoin::Address::PublicKey(winner.address.clone()),
            miner.burn_per_block,
        );
        debit(
            api,
            ellipticoin::Address::PublicKey(miner.address.clone()),
            miner.burn_per_block,
        )?;
    }
    Ok(())
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

fn credit_issuance_rewards<API: ellipticoin::API>(api: &mut API, address: Address, amount: u64) {
    let issuance_rewards = get_issuance_rewards(api, address.clone());
    set_issuance_rewards(api, address, issuance_rewards + amount);
}

fn debit_issuance_rewards<API: ellipticoin::API>(api: &mut API, address: Address, amount: u64) {
    let issuance_rewards = get_issuance_rewards(api, address.clone());
    set_issuance_rewards(api, address, issuance_rewards - amount);
}

fn distribute(mut amount: u64, mut values: Vec<u64>) -> Vec<u64> {
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
            exchange::constants::BASE_TOKEN,
            test_api::{TestAPI, TestState},
            token::{constants::ETH, get_balance, BASE_FACTOR},
        },
    };
    use std::env;

    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB, CAROL};

    #[test]
    fn test_harvest() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        env::set_var("HOST", "localhost");
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Ellipticoin".to_string());
        mint(&mut api, ELC.clone(), Address::Contract(ADDRESS.clone()), 1).unwrap();
        credit_issuance_rewards(&mut api, Address::PublicKey(*ALICE), 1);
        native::harvest(&mut api);
        assert_eq!(
            get_balance(&mut api, ELC.clone(), Address::PublicKey(*ALICE)),
            1
        );
    }

    #[test]
    fn test_whitelist_miner() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        env::set_var("HOST", "localhost");
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Ellipticoin".to_string());

        let alice_pub: [u8; 32] = Address::PublicKey(*ALICE).as_public_key().unwrap();
        let bob_pub: [u8; 32] = Address::PublicKey(*BOB).as_public_key().unwrap();
        let carol_pub: [u8; 32] = Address::PublicKey(*CAROL).as_public_key().unwrap();
        native::whitelist_miner(&mut api, bob_pub).expect("whitelisting bob failed!");

        let mut whitelist = get_miner_whitelist(&mut api);
        assert!(whitelist.contains(&alice_pub), "Alice's address not present in whitelist!");
        assert!(whitelist.contains(&bob_pub), "Bob's address not present in whitelist!");
        assert!(!whitelist.contains(&carol_pub), "Carol's address present in whitelist when it shouldn't be!");

        native::whitelist_miner(&mut api, carol_pub).expect("Whitelisting carol failed!");
        whitelist = get_miner_whitelist(&mut api);

        assert!(whitelist.contains(&carol_pub), "Carol's address not present in whitelist!");
    }

    #[test]
    #[should_panic]
    fn test_whitelist_miner_failure() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        env::set_var("HOST", "localhost");
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Ellipticoin".to_string());

        let alice_pub: [u8; 32] = Address::PublicKey(*ALICE).as_public_key().unwrap();
        let bob_pub: [u8; 32] = Address::PublicKey(*BOB).as_public_key().unwrap();
        let carol_pub: [u8; 32] = Address::PublicKey(*CAROL).as_public_key().unwrap();
        native::whitelist_miner(&mut api, bob_pub).expect("whitelisting bob failed!");

        let mut whitelist = get_miner_whitelist(&mut api);
        assert!(whitelist.contains(&alice_pub), "Alice's address not present in whitelist!");
        assert!(whitelist.contains(&bob_pub), "Bob's address not present in whitelist!");
        assert!(!whitelist.contains(&carol_pub), "Carol's address present in whitelist when it shouldn't be!");

        api.caller = Address::PublicKey(*CAROL);
        native::whitelist_miner(&mut api, bob_pub).expect("This should fail");
    }

    #[test]
    fn test_commit_and_seal() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        env::set_var("HOST", "localhost");
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Ellipticoin".to_string());
        credit(&mut api, Address::PublicKey(*ALICE), 5);
        credit(&mut api, Address::PublicKey(*BOB), 5);
        let alices_center = [0; 32];
        let bobs_center = [1; 32];
        let mut alices_onion = generate_hash_onion(3, alices_center.clone());
        let mut bobs_onion = generate_hash_onion(3, bobs_center.clone());
        token::set_balance(
            &mut api,
            ETH.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            1 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            1 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            ETH.clone(),
            ellipticoin::Address::PublicKey(*BOB),
            1 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone(),
            ellipticoin::Address::PublicKey(*BOB),
            1 * BASE_FACTOR,
        );
        api.caller = Address::PublicKey(*ALICE);
        exchange::native::create_pool(&mut api, ETH.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR)
            .unwrap();
        api.caller = Address::PublicKey(*BOB);
        exchange::native::add_liqidity(&mut api, ETH.clone(), 1 * BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(*ALICE);
        native::start_mining(&mut api, HOST.to_string(), 1, *alices_onion.last().unwrap()).unwrap();
        api.caller = Address::PublicKey(*BOB);
        native::start_mining(&mut api, HOST.to_string(), 1, *bobs_onion.last().unwrap()).unwrap();

        // With this random seed the winners are Alice, Alice, Bob in that order
        api.caller = Address::PublicKey(*ALICE);
        alices_onion.pop();
        assert!(native::seal(&mut api, *alices_onion.last().unwrap()).is_ok());
        alices_onion.pop();
        assert!(native::seal(&mut api, *alices_onion.last().unwrap()).is_ok());
        api.caller = Address::PublicKey(*BOB);
        bobs_onion.pop();
        assert!(native::seal(&mut api, *bobs_onion.last().unwrap()).is_ok());
        assert_eq!(
            get_balance(&mut api, ELC.clone(), Address::PublicKey(*ALICE)),
            6
        );
        assert_eq!(
            get_balance(&mut api, ELC.clone(), Address::PublicKey(*BOB)),
            4
        );
        assert_eq!(
            get_balance(&mut api, ELC.clone(), Address::PublicKey(*BOB)),
            4
        );
        assert_eq!(
            get_issuance_rewards(&mut api, Address::PublicKey(*ALICE)),
            960_000
        );
        assert_eq!(
            get_issuance_rewards(&mut api, Address::PublicKey(*BOB)),
            960_000
        );
        assert_eq!(get_block_number(&mut api), 3);
    }
}
