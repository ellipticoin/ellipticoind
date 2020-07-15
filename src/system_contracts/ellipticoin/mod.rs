mod errors;
mod ethereum;
mod hashing;

use crate::helpers::zero_pad_vec;
use ellipticoin::{constants::SYSTEM_ADDRESS, Address};
use errors::Error;
use hashing::sha256;
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_cbor::{value::to_value, Value};
use std::convert::TryInto;
use wasm_rpc_macros::export_native;

enum Namespace {
    _Allowances,
    #[cfg(test)]
    _Balances,
    #[cfg(not(test))]
    _Balances,
    BlockNumber,
    EthereumBalances,
    Miners,
    UnlockedEthereumBalances,
    TotalUnlockedEthereum,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Miner {
    pub host: String,
    pub address: Address,
    pub burn_per_block: u64,
    pub hash_onion_skin: [u8; 32],
}

export_native! {
    pub fn transfer_to_current_miner<API: ellipticoin::API>(api: &mut API, amount: u64) -> Result<u64, Box<Error>> {
        if get_balance(api, api.caller()) < amount {
            return Err(Box::new(errors::INSUFFICIENT_FUNDS.clone()));
        }

        let miners = get_miners(api, );
        debit(api, api.caller(), amount);
        credit(api, miners.first().unwrap().address.clone(), amount);
        Ok(get_balance(api, api.caller()).into())
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
        if api.get_storage(&[[Namespace::UnlockedEthereumBalances as u8].to_vec(), address.clone()].concat()).unwrap_or(false) {
            return Err(Box::new(errors::BALANCE_ALREADY_UNLOCKED.clone()));
        };
        let balance: u64 =
            api.get_storage::<u64>(&[[Namespace::EthereumBalances as u8].to_vec(), address.clone()].concat()).unwrap_or(0) * 100;

        let mut total_unlocked_ethereum: u64 =
            api.get_storage::<u64>(&[Namespace::TotalUnlockedEthereum as u8].to_vec()).unwrap_or(0);
        if total_unlocked_ethereum + balance > 1000000 * 10000 {
            return Err(Box::new(errors::BALANCE_EXCEEDS_THIS_PHASE.clone()));
        } else {
            total_unlocked_ethereum += balance;
            api.set_storage::<u64>(
                &[Namespace::TotalUnlockedEthereum as u8].to_vec(),
                total_unlocked_ethereum,
            );
        }
        credit(api, Address::PublicKey(ellipticoin_address), balance);
        api.set_storage(&[
            [Namespace::UnlockedEthereumBalances as u8].to_vec(),
            address].concat(),
            true
        );

        Ok(balance.into())
    }

    pub fn start_mining<API: ellipticoin::API>(
        api: &mut API,
        host: String,
        burn_per_block: u64,
        hash_onion_skin: [u8; 32],
    ) -> Result<Value, Box<Error>> {
        let mut miners = get_miners(api, );
        miners.push(Miner {
            address: api.caller(),
            host,
            burn_per_block,
            hash_onion_skin,
        });
        set_miners(api, &miners);
        Ok(Value::Null)
    }

    pub fn reveal<API: ellipticoin::API>(api: &mut API, value: [u8; 32]) -> Result<Value, Box<Error>> {
        let mut miners = get_miners(api, );
        if api.caller() != miners.first().unwrap().address {
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
        settle_block_rewards(api);
        miners.first_mut().unwrap().hash_onion_skin = value.clone();
        shuffle_miners(api, miners, value);
        increment_block_number(api);

        Ok(Value::Null)
    }

    fn increment_block_number<API: ellipticoin::API>(api: &mut API) {
        let block_number = api.get_storage::<u32>(&[Namespace::BlockNumber as u8]).unwrap_or(0);
        api.set_storage::<u32>(&[Namespace::BlockNumber as u8], block_number + 1);
    }

    fn shuffle_miners<API: ellipticoin::API>(api: &mut API, mut miners: Vec<Miner>, value: [u8; 32]) {
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
        set_miners(api, &shuffled_miners)
    }

    pub fn get_miners<API: ellipticoin::API>(api: &mut API) -> Vec<Miner> {
        api.get_storage::<Vec<Miner>>(&[Namespace::Miners as u8]).unwrap_or(vec![])
    }

    fn set_miners<API: ellipticoin::API>(api: &mut API, miners: &Vec<Miner>) {
        api.set_storage::<Value>(&[Namespace::Miners as u8], to_value(miners).unwrap());
    }

    fn settle_block_rewards<API: ellipticoin::API>(api: &mut API) {
        let miners = get_miners(api);
        let winner = miners.first().as_ref().unwrap().clone();
        for miner in &miners {
            credit(api, winner.address.clone(), miner.burn_per_block);
            debit(api, miner.address.clone(), miner.burn_per_block);
        }
    }

    fn get_balance<API: ellipticoin::API>(api: &mut API, address: Address) -> u64 {
        let token_id: [u8; 32] = zero_pad_vec("ELC".as_bytes(), 32)[..].try_into().unwrap();
        api.call(
            SYSTEM_ADDRESS,
            "Token",
            "get_balance",
            vec![
                serde_cbor::value::to_value(Address::Contract(SYSTEM_ADDRESS, "Ellipticoin".to_string())).unwrap(),
                serde_cbor::value::to_value(token_id).unwrap(),
                to_value(address).unwrap()
            ]).unwrap_or(0)
    }

    fn credit<API: ellipticoin::API>(api: &mut API, address: Address, amount: u64) {
        let token_id: [u8; 32] = zero_pad_vec("ELC".as_bytes(), 32)[..].try_into().unwrap();
        api.call::<Value>(
            SYSTEM_ADDRESS,
            "Token",
            "credit",
            vec![
                serde_cbor::value::to_value(Address::Contract(SYSTEM_ADDRESS, "Ellipticoin".to_string())).unwrap(),
                serde_cbor::value::to_value(token_id).unwrap(),
                to_value(address).unwrap(),
                to_value(amount).unwrap()
            ]).unwrap();
    }

    fn debit<API: ellipticoin::API>(api: &mut API, address: Address, amount: u64) {
        let token_id: [u8; 32] = zero_pad_vec("ELC".as_bytes(), 32)[..].try_into().unwrap();
        api.call::<Value>(
            SYSTEM_ADDRESS,
            "Token",
            "debit",
            vec![
                serde_cbor::value::to_value(Address::Contract(SYSTEM_ADDRESS, "Ellipticoin".to_string())).unwrap(),
                serde_cbor::value::to_value(token_id).unwrap(),
                to_value(address).unwrap(),
                to_value(amount).unwrap()
            ]).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::HOST,
        helpers::generate_hash_onion,
        system_contracts::api::{TestAPI, TestState},
    };
    use ellipticoin::API;
    use std::env;

    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB};

    // #[test]
    // fn test_transfer() {
    //     env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
    //     let mut state = TestState::new();
    //     let mut api = TestAPI::new(&mut state, *ALICE);
    //     set_balance(&mut api, *ALICE, 100);
    //     let result = transfer(&mut api, **BOB, 20).unwrap();
    //     assert_eq!(result, Value::Integer(80u8.into()));
    //     assert_eq!(get_balance(&mut api, &ALICE[..]), 80);
    //     assert_eq!(get_balance(&mut api, &*BOB[..]), 20);
    // }

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
        start_mining(&mut api, HOST.to_string(), 1, *alices_onion.last().unwrap()).unwrap();
        api.caller = Address::PublicKey(*BOB);
        start_mining(&mut api, HOST.to_string(), 1, *bobs_onion.last().unwrap()).unwrap();

        // With this random seed the winners are Alice, Alice, Bob in that order
        api.caller = Address::PublicKey(*ALICE);
        alices_onion.pop();
        assert!(reveal(&mut api, *alices_onion.last().unwrap()).is_ok());
        alices_onion.pop();
        assert!(reveal(&mut api, *alices_onion.last().unwrap()).is_ok());
        api.caller = Address::PublicKey(*BOB);
        bobs_onion.pop();
        assert!(reveal(&mut api, *bobs_onion.last().unwrap()).is_ok());

        assert_eq!(get_balance(&mut api, Address::PublicKey(*ALICE)), 6);
        assert_eq!(get_balance(&mut api, Address::PublicKey(*BOB)), 4);
        assert_eq!(
            api.get_storage::<u64>(&[Namespace::BlockNumber as u8][..])
                .unwrap(),
            3
        );
    }
}
