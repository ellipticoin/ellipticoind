use ellipticoin::{
    caller,
    error::Error,
    export,
    value::{from_value, to_value},
    FromBytes, ToBytes, Value,
};
use errors;
use ethereum;
use hashing::sha256;
use wasm_rpc::serde::{Deserialize, Serialize};

enum Namespace {
    Allowances,
    Balances,
    BlockNumber,
    EthereumBalances,
    Miners,
    UnlockedEthereumBalances,
    TotalUnlockedEthereum,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
struct Miner {
    host: String,
    address: Vec<u8>,
    burn_per_block: u64,
    hash_onion_skin: Vec<u8>,
}

use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use std::convert::TryInto;

#[export]
mod token {
    pub fn approve(spender: Vec<u8>, amount: u64) {
        set_memory(
            Namespace::Allowances,
            [caller(), spender.to_vec()].concat(),
            amount,
        );
    }

    pub fn transfer_to_current_miner(amount: u64) -> Result<Value, Error> {
        if get_balance(caller().clone()) < amount {
            return Err(errors::INSUFFICIENT_FUNDS);
        }

        debit(caller(), amount);
        credit(get_miners().first().unwrap().address.clone(), amount);
        Ok(get_balance(caller()).into())
    }

    pub fn transfer_from(from: Vec<u8>, to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        if get_allowance(from.clone(), caller()) < amount {
            return Err(errors::INSUFFICIENT_ALLOWANCE);
        }

        if get_balance(from.clone()) < amount {
            return Err(errors::INSUFFICIENT_FUNDS);
        }

        debit_allowance(from.clone().to_vec(), caller(), amount);
        debit(from.to_vec(), amount);
        credit(to.to_vec(), amount);
        Ok(Value::Null)
    }

    pub fn transfer(to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        if get_balance(caller()) >= amount {
            debit(caller(), amount);
            credit(to.to_vec(), amount);
            Ok(get_balance(caller()).into())
        } else {
            Err(errors::INSUFFICIENT_FUNDS)
        }
    }

    fn debit_allowance(from: Vec<u8>, to: Vec<u8>, amount: u64) {
        let allowance = get_allowance(from.clone(), to.clone());
        set_allowance(from, to, allowance - amount);
    }

    pub fn unlock_ether(
        unlock_signature: Vec<u8>,
        ellipticoin_address: Vec<u8>,
    ) -> Result<Value, Error> {
        let encoded_ellipticoin_adress =
            base64::encode_config(&ellipticoin_address, base64::URL_SAFE_NO_PAD);
        let message = format!(
            "Unlock Ellipticoin at address: {}",
            encoded_ellipticoin_adress
        );
        let address = ethereum::ecrecover_address(message.as_bytes(), &unlock_signature);
        if get_storage(Namespace::UnlockedEthereumBalances, address.clone()) {
            return Err(errors::BALANCE_ALREADY_UNLOCKED);
        };
        let balance: u64 =
            get_storage::<_, u64>(Namespace::EthereumBalances, address.clone()) * 100;

        let mut total_unlocked_ethereum: u64 =
            get_storage::<Vec<u8>, u64>(Namespace::TotalUnlockedEthereum, vec![]);
        if total_unlocked_ethereum + balance > 1000000 * 10000 {
            return Err(errors::BALANCE_EXCEEDS_THIS_PHASE);
        } else {
            total_unlocked_ethereum += balance;
            set_storage::<Vec<u8>, u64>(
                Namespace::TotalUnlockedEthereum,
                vec![],
                total_unlocked_ethereum,
            );
        }
        credit(ellipticoin_address, balance);
        set_storage(Namespace::UnlockedEthereumBalances, address.clone(), true);

        Ok(balance.into())
    }

    pub fn start_mining(
        host: String,
        burn_per_block: u64,
        hash_onion_skin: Vec<u8>,
    ) -> Result<Value, Error> {
        let mut miners = get_miners();
        miners.push(Miner {
            address: caller(),
            host,
            burn_per_block,
            hash_onion_skin,
        });
        set_miners(&miners);
        Ok(Value::Null)
    }

    pub fn reveal(value: Vec<u8>) -> Result<Value, Error> {
        let mut miners = get_miners();
        if caller() != miners.first().unwrap().address {
            return Err(errors::SENDER_IS_NOT_THE_WINNER);
        }
        if !miners
            .first()
            .unwrap()
            .hash_onion_skin
            .eq(&sha256(value.clone()))
        {
            return Err(errors::INVALID_VALUE);
        }
        settle_block_rewards();
        miners.first_mut().unwrap().hash_onion_skin = value.clone();
        set_miners(&shuffle_miners(miners, value));
        increment_block_number();

        Ok(Value::Null)
    }

    fn increment_block_number() {
        let block_number = ellipticoin::get_storage::<_, u32>(Namespace::BlockNumber as u8);
        ellipticoin::set_storage::<_, u32>(Namespace::BlockNumber as u8, block_number + 1);
    }

    fn shuffle_miners(mut miners: Vec<Miner>, value: Vec<u8>) -> Vec<Miner> {
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
        shuffled_miners
    }
    fn get_miners() -> Vec<Miner> {
        from_value(ellipticoin::get_storage(Namespace::Miners as u8)).unwrap_or(vec![])
    }

    fn set_miners(miners: &Vec<Miner>) {
        ellipticoin::set_storage::<_, Value>(Namespace::Miners as u8, to_value(miners).unwrap());
    }

    fn settle_block_rewards() {
        let miners = get_miners();
        let winner = miners.first().as_ref().unwrap().clone();
        for miner in &miners {
            credit(winner.address.to_vec(), miner.burn_per_block);
            debit(miner.address.to_vec(), miner.burn_per_block);
        }
    }

    fn get_balance(address: Vec<u8>) -> u64 {
        get_memory::<_, u64>(Namespace::Balances, address)
    }

    fn get_allowance(from: Vec<u8>, to: Vec<u8>) -> u64 {
        get_memory(
            Namespace::Allowances,
            [from.clone().to_vec(), to.clone().to_vec()].concat(),
        )
    }

    fn set_allowance(from: Vec<u8>, to: Vec<u8>, value: u64) {
        set_memory(
            Namespace::Allowances,
            [from.to_vec(), to.to_vec()].concat(),
            value,
        );
    }

    fn credit(address: Vec<u8>, amount: u64) {
        let balance: u64 = get_memory(Namespace::Balances, address.clone());
        set_memory(Namespace::Balances, address, balance + amount);
    }

    fn debit(address: Vec<u8>, amount: u64) {
        let balance: u64 = get_memory(Namespace::Balances, address.clone());
        set_memory(Namespace::Balances, address, balance - amount);
    }

    fn set_memory<K: ToBytes, V: ToBytes>(namespace: Namespace, key: K, value: V) {
        ellipticoin::set_memory([vec![namespace as u8], key.to_bytes()].concat(), value);
    }

    fn get_memory<K: ToBytes, V: FromBytes>(namespace: Namespace, key: K) -> V {
        ellipticoin::get_memory([vec![namespace as u8], key.to_bytes()].concat())
    }

    fn get_storage<K: ToBytes, V: FromBytes>(namespace: Namespace, key: K) -> V {
        ellipticoin::get_storage([vec![namespace as u8], key.to_bytes()].concat())
    }

    fn set_storage<K: ToBytes, V: ToBytes>(namespace: Namespace, key: K, value: V) {
        ellipticoin::set_storage([vec![namespace as u8], key.to_bytes()].concat(), value);
    }
}

#[cfg(test)]
mod tests {
    const HOST: &'static str = "localhost";
    use super::*;
    use ellipticoin::{set_block_number, set_caller};
    use ellipticoin_test_framework::{
        generate_hash_onion, random_bytes, sha256, ALICE, BOB, CAROL,
    };

    #[test]
    fn test_transfer() {
        set_caller(ALICE.to_vec());
        set_balance(ALICE.to_vec(), 100);
        let result = transfer(BOB.to_vec(), 20).unwrap();
        assert_eq!(result, Value::Integer(80));
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 80);
        let bobs_balance = balance_of(BOB.to_vec());
        assert_eq!(bobs_balance, 20);
    }

    #[test]
    fn test_transfer_to_current_miner() {
        set_balance(ALICE.to_vec(), 100);
        set_caller(BOB.to_vec());
        start_mining(HOST.to_string(), 1, vec![]).unwrap();
        set_caller(ALICE.to_vec());
        let result = transfer_to_current_miner(20).unwrap();
        assert_eq!(result, Value::Integer(80));
        assert_eq!(balance_of(ALICE.to_vec()), 80);
        assert_eq!(balance_of(BOB.to_vec()), 20);
    }

    #[test]
    fn test_transfer_insufficient_funds() {
        set_caller(ALICE.to_vec());
        start_mining(HOST.to_string(), 1, vec![]).unwrap();
        set_balance(ALICE.to_vec(), 100);
        assert!(transfer(BOB.to_vec(), 120).is_err());
    }

    #[test]
    fn test_transfer_from_insufficient_funds() {
        set_caller(ALICE.to_vec());
        start_mining(HOST.to_string(), 1, vec![]).unwrap();
        set_balance(BOB.to_vec(), 100);
        assert!(transfer_from(BOB.to_vec(), CAROL.to_vec(), 120).is_err());
    }

    #[test]
    fn test_transfer_from() {
        set_caller(ALICE.to_vec());
        set_balance(ALICE.to_vec(), 100);
        approve(BOB.to_vec(), 50);
        set_caller(BOB.to_vec());
        transfer_from(ALICE.to_vec(), CAROL.to_vec(), 20).unwrap();
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 80);
        let bobs_allowance = allowance(ALICE.to_vec(), BOB.to_vec());
        assert_eq!(bobs_allowance, 30);
        let carols_balance = balance_of(CAROL.to_vec());
        assert_eq!(carols_balance, 20);
    }

    pub fn set_balance(address: Vec<u8>, balance: u64) {
        set_memory(Namespace::Balances, address, balance);
    }

    pub fn balance_of(address: Vec<u8>) -> u64 {
        get_memory(Namespace::Balances, address)
    }

    pub fn allowance(owner: Vec<u8>, spender: Vec<u8>) -> u64 {
        get_memory(Namespace::Allowances, [owner, spender].concat())
    }

    #[test]
    fn test_unlock_ether() {
        let ethereum_address = "adfe2b5beac83382c047d977db1df977fd9a7e41";
        let signature = hex::decode(&"e8fe080305be6153dda25cd046f022fe93fce9e9abf7443cb602236317769ea3007922a1ee66a8dc64caae93bd7073af95633bb64389b61679c83c05590d1fbf1c").unwrap();
        set_caller(ALICE.to_vec());
        set_storage(
            Namespace::EthereumBalances,
            hex::decode(ethereum_address).unwrap(),
            1000 as u64,
        );
        unlock_ether(signature, ALICE.to_vec()).unwrap();
        assert_eq!(balance_of(ALICE.to_vec()), 100000);
    }

    #[test]
    fn test_unlock_ether_legacy_signature() {
        let ellipticoin_address =
            hex::decode(&"7075499ca17b8459fd6ca407d4769ea51bc27edf85fe75e003e4f5f786478749")
                .unwrap()
                .to_vec();
        let ethereum_address = "a3953352beab67861e5a7fff47f36611a1ff5335";
        let signature = hex::decode(&"0578e0ba123dcf2e563cb2fb5937d01bc4c1e12d4147a8ea231d3b595b0890a4057338c379cd55b71160a7aa301642c972ba1d519b7c74b2254d8c0ebbb9bf1500").unwrap();
        set_caller(ALICE.to_vec());
        set_storage(
            Namespace::EthereumBalances,
            hex::decode(ethereum_address).unwrap(),
            1000 as u64,
        );
        unlock_ether(signature, ellipticoin_address.clone()).unwrap();
        assert_eq!(balance_of(ellipticoin_address), 100000);
    }

    #[test]
    fn test_unlock_ether_twice() {
        let ethereum_address = "adfe2b5beac83382c047d977db1df977fd9a7e41";
        set_caller(ALICE.to_vec());
        set_storage(
            Namespace::EthereumBalances,
            hex::decode(ethereum_address).unwrap(),
            1000 as u64,
        );
        unlock_ether(hex::decode(&"e8fe080305be6153dda25cd046f022fe93fce9e9abf7443cb602236317769ea3007922a1ee66a8dc64caae93bd7073af95633bb64389b61679c83c05590d1fbf1c").unwrap(), ALICE.to_vec()).unwrap();
        assert!(unlock_ether(hex::decode(&"e8fe080305be6153dda25cd046f022fe93fce9e9abf7443cb602236317769ea3007922a1ee66a8dc64caae93bd7073af95633bb64389b61679c83c05590d1fbf1c").unwrap(), ALICE.to_vec()).is_err());
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 100000);
    }

    #[test]
    fn test_unlock_ether_over_limit() {
        let ethereum_address = "adfe2b5beac83382c047d977db1df977fd9a7e41";
        set_caller(ALICE.to_vec());
        set_storage(
            Namespace::EthereumBalances,
            hex::decode(ethereum_address).unwrap(),
            100000001 as u64,
        );
        assert!(unlock_ether(hex::decode(&"e8fe080305be6153dda25cd046f022fe93fce9e9abf7443cb602236317769ea3007922a1ee66a8dc64caae93bd7073af95633bb64389b61679c83c05590d1fbf1c").unwrap(), ALICE.to_vec()).is_err());
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 0);
    }

    #[test]
    fn test_commit_and_reveal() {
        set_balance(ALICE.to_vec(), 5);
        set_balance(BOB.to_vec(), 5);
        let alices_center = [0; 32].to_vec();
        let bobs_center = [1; 32].to_vec();
        let mut alices_onion = generate_hash_onion(3, alices_center.clone());
        let mut bobs_onion = generate_hash_onion(3, bobs_center.clone());
        set_caller(ALICE.to_vec());
        start_mining(HOST.to_string(), 1, alices_onion.last().unwrap().to_vec()).unwrap();
        set_caller(BOB.to_vec());
        start_mining(HOST.to_string(), 1, bobs_onion.last().unwrap().to_vec()).unwrap();

        // With this random seed the winners are rlice, rlice, Bob in that order
        set_caller(ALICE.to_vec());
        alices_onion.pop();
        assert!(reveal(alices_onion.last().unwrap().to_vec()).is_ok());
        alices_onion.pop();
        assert!(reveal(alices_onion.last().unwrap().to_vec()).is_ok());
        set_caller(BOB.to_vec());
        bobs_onion.pop();
        assert!(reveal(bobs_onion.last().unwrap().to_vec()).is_ok());

        assert_eq!(balance_of(ALICE.to_vec()), 6);
        assert_eq!(balance_of(BOB.to_vec()), 4);
        assert_eq!(
            get_storage::<Vec<u8>, u32>(Namespace::BlockNumber, vec![]),
            3
        );
    }

    #[test]
    fn test_commit_and_reveal_invalid() {
        let value = random_bytes(32);
        let hash = sha256(value.clone());
        let invalid_value = random_bytes(32);
        set_caller(ALICE.to_vec());

        start_mining(HOST.to_string(), 1, hash).unwrap();
        set_block_number(1);
        assert!(reveal(invalid_value).is_err());
    }
}
