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

use std::collections::BTreeMap;
enum Namespace {
    Allowences,
    Balances,
    CurrentMiner,
    EthereumBalances,
    Miners,
    RandomSeed,
    UnlockedEthereumBalances,
    TotalUnlockedEthereum,
}

use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use std::convert::TryInto;

#[export]
mod token {
    pub fn approve(spender: Vec<u8>, amount: u64) {
        set_memory(
            Namespace::Allowences,
            [caller(), spender.to_vec()].concat(),
            amount,
        );
    }

    pub fn transfer_from(from: Vec<u8>, to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        let allowance: u64 = get_memory(
            Namespace::Allowences,
            [from.clone().to_vec(), caller()].concat(),
        );

        if get_memory::<_, u64>(Namespace::Balances, from.clone()) < amount {
            return Err(errors::INSUFFICIENT_FUNDS);
        }

        if allowance >= amount {
            debit_allowance(from.clone().to_vec(), caller(), amount);
            debit(from.to_vec(), amount);
            credit(to.to_vec(), amount);
            Ok(Value::Null)
        } else {
            Err(errors::INSUFFICIENT_FUNDS)
        }
    }

    pub fn transfer(to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        if get_memory::<_, u64>(Namespace::Balances, caller()) >= amount {
            debit(caller(), amount);
            credit(to.to_vec(), amount);
            Ok(Value::Null)
        } else {
            Err(errors::INSUFFICIENT_FUNDS)
        }
    }

    fn debit_allowance(from: Vec<u8>, to: Vec<u8>, amount: u64) {
        let allowance: u64 = get_memory(
            Namespace::Allowences,
            [from.clone().to_vec(), to.clone().to_vec()].concat(),
        );

        set_memory(
            Namespace::Allowences,
            [from.to_vec(), to.to_vec()].concat(),
            allowance - amount,
        );
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

        let mut total_unlocked_ethereum: u64 = get_storage::<Vec<u8>, u64>(Namespace::TotalUnlockedEthereum, vec![]);
        if  total_unlocked_ethereum + balance > 1000000 * 10000 {
            return Err(errors::BALANCE_EXCEEDS_THIS_PHASE);
        } else {
            total_unlocked_ethereum += balance;
            set_storage::<Vec<u8>, u64>(Namespace::TotalUnlockedEthereum, vec![], total_unlocked_ethereum);
        }
        credit(ellipticoin_address, balance);
        set_storage(Namespace::UnlockedEthereumBalances, address.clone(), true);

        Ok(balance.into())
    }

    pub fn start_mining(
        host: String,
        bet_per_block: u64,
        hash_onion: Vec<u8>,
    ) -> Result<Value, Error> {
        let mut miners = get_miners();
        miners.insert(caller(), (host, bet_per_block, hash_onion.clone().to_vec()));
        set_miners(&miners);
        Ok(Value::Null)
    }

    pub fn reveal(value: Vec<u8>) -> Result<Value, Error> {
        let mut miners = get_miners();
        if caller() != get_current_miner() {
            return Err(errors::SENDER_IS_NOT_THE_WINNER);
        }
        let (host, bet_per_block, hash) = miners.get(&caller()).unwrap();
        if !hash.to_vec().eq(&sha256(value.clone().to_vec())) {
            return Err(errors::INVALID_VALUE);
        }
        *miners.get_mut(&caller()).unwrap() = (host.clone(), *bet_per_block, value.clone());
        settle_block_rewards(get_current_miner(), &miners);
        let random_seed = get_random_seed();
        set_random_seed(
            sha256([random_seed.to_vec(), value.to_vec()].concat())[0..16]
            .try_into()
            .unwrap(),
            );
        set_miners(&miners);
        set_current_miner(get_next_winner(&miners).to_vec());

        Ok(Value::Null)
    }

    fn set_current_miner(current_miner: Vec<u8>) {
        ellipticoin::set_storage::<_, Vec<u8>>(
            Namespace::CurrentMiner as u8,
            current_miner.to_vec(),
        );
    }

    fn get_current_miner() -> Vec<u8> {
        ellipticoin::get_storage::<_, Vec<u8>>(Namespace::CurrentMiner as u8).to_vec()
    }

    fn set_random_seed(random_seed: [u8; 16]) {
        ellipticoin::set_storage(Namespace::RandomSeed as u8, random_seed.to_vec());
    }
    fn get_random_seed() -> [u8; 16] {
        let random_seed = ellipticoin::get_storage::<_, Vec<u8>>(Namespace::RandomSeed as u8);

        if random_seed.len() == 0 {
            [0 as u8; 16].try_into().unwrap()
        } else {
            random_seed[0..16].try_into().unwrap()
        }
    }

    fn get_next_winner(miners: &BTreeMap<Vec<u8>, (String, u64, Vec<u8>)>) -> Vec<u8> {
        let random_seed: [u8; 16] = get_random_seed();
        let mut rng = SmallRng::from_seed(random_seed);
        let mut bets: Vec<(Vec<u8>, u64)> = miners
            .iter()
            .map(|(miner, (_host, bet_per_block, _hash))| (miner.to_vec(), *bet_per_block))
            .collect();
        bets.sort();
        bets.choose_weighted(&mut rng, |(_miner, bet_per_block)| *bet_per_block)
            .map(|(miner, _bet_per_block)| miner.to_vec())
            .unwrap()
    }

    fn get_miners() -> BTreeMap<Vec<u8>, (String, u64, Vec<u8>)> {
        from_value(ellipticoin::get_storage(Namespace::Miners as u8)).unwrap_or(BTreeMap::new())
    }

    fn set_miners(miners: &BTreeMap<Vec<u8>, (String, u64, Vec<u8>)>) {
        ellipticoin::set_storage(
            Namespace::Miners as u8,
            to_value(miners).unwrap_or(Value::Null),
        );
    }

    fn settle_block_rewards(winner: Vec<u8>, miners: &BTreeMap<Vec<u8>, (String, u64, Vec<u8>)>) {
        for (miner, (_host, bet_per_block, _hash)) in miners {
            if miner.to_vec() != winner.to_vec() {
                credit(winner.to_vec(), *bet_per_block);
                debit(miner.to_vec(), *bet_per_block);
            }
        }
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
        transfer(BOB.to_vec(), 20).unwrap();
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 80);
        let bobs_balance = balance_of(BOB.to_vec());
        assert_eq!(bobs_balance, 20);
    }

    #[test]
    fn test_transfer_insufficient_funds() {
        set_caller(ALICE.to_vec());
        set_balance(ALICE.to_vec(), 100);
        assert!(transfer(BOB.to_vec(), 120).is_err());
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
        get_memory(Namespace::Allowences, [owner, spender].concat())
    }

    #[test]
    fn test_unlock_ether() {
        let ethereum_address = "adfe2b5beac83382c047d977db1df977fd9a7e41";
        set_caller(ALICE.to_vec());
        set_storage(
            Namespace::EthereumBalances,
            hex::decode(ethereum_address).unwrap(),
            1000 as u64,
        );
        unlock_ether(hex::decode(&"e8fe080305be6153dda25cd046f022fe93fce9e9abf7443cb602236317769ea3007922a1ee66a8dc64caae93bd7073af95633bb64389b61679c83c05590d1fbf1c").unwrap(), ALICE.to_vec()).unwrap();
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 100000);
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
        set_random_seed([2 as u8; 16]);
        set_balance(ALICE.to_vec(), 5);
        set_balance(BOB.to_vec(), 5);
        set_current_miner(ALICE.to_vec());
        let alices_center = [0; 32].to_vec();
        let bobs_center = [1; 32].to_vec();
        let mut alices_onion = generate_hash_onion(3, alices_center.clone());
        let mut bobs_onion = generate_hash_onion(3, bobs_center.clone());
        set_caller(ALICE.to_vec());
        start_mining(HOST.to_string(), 1, alices_onion.last().unwrap().to_vec()).unwrap();
        set_caller(BOB.to_vec());
        start_mining(HOST.to_string(), 1, bobs_onion.last().unwrap().to_vec()).unwrap();

        // With this random seed the winners are Alice, Alice, Bob in that order
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
    }

    #[test]
    fn test_commit_and_reveal_invalid() {
        set_random_seed([0 as u8; 16]);
        let value = random_bytes(32);
        let hash = sha256(value.clone());
        let invalid_value = random_bytes(32);
        set_caller(ALICE.to_vec());

        start_mining(HOST.to_string(), 1, hash).unwrap();
        set_block_number(1);
        assert!(reveal(invalid_value).is_err());
    }
}
