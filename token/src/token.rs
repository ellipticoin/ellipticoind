use ellipticoin::abort::{AbortOptionExt, AbortResultExt};
use ellipticoin::{
    error::Error,
    export, sender,
    value::{from_value, to_value},
    FromBytes, ToBytes, Value,
};
use errors;
use ethereum;
use hashing::sha256;
use std::collections::HashMap;
enum Namespace {
    Allowences,
    Balances,
    CurrentMiner,
    Miners,
    RandomSeed,
    EthereumBalances,
}

use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use std::convert::TryInto;

#[export]
mod token {
    pub fn approve(spender: Vec<u8>, amount: u64) {
        set_memory(
            Namespace::Allowences,
            [sender(), spender.to_vec()].concat(),
            amount,
        );
    }

    pub fn transfer_from(from: Vec<u8>, to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        let allowance: u64 = get_memory(
            Namespace::Allowences,
            [from.clone().to_vec(), sender()].concat(),
        );

        if allowance >= amount {
            debit_allowance(from.clone().to_vec(), sender(), amount);
            debit(from.to_vec(), amount);
            credit(to.to_vec(), amount);
            Ok(Value::Null)
        } else {
            Err(errors::INSUFFICIENT_FUNDS)
        }
    }

    pub fn transfer(to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        if get_memory::<_, u64>(Namespace::Balances, sender()) >= amount {
            debit(sender(), amount);
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

    pub fn unlock(unlock_signature: Vec<u8>) {
        let message = "unlock_ellipticoin";
        let address = ethereum::ecrecover_address(message.as_bytes(), &unlock_signature);
        let balance = get_memory(Namespace::EthereumBalances, address);
        credit(sender(), balance);
    }

    pub fn start_mining(bet_per_block: u64, hash_onion: Vec<u8>) -> Result<Value, Error> {
        let mut miners = get_miners();
        miners.insert(
            sender(),
            (bet_per_block, hash_onion.clone().to_vec()),
        );
        set_miners(&miners);
        Ok(Value::Null)
    }

    pub fn reveal(value: Vec<u8>) -> Result<Value, Error> {
        let mut miners = get_miners();
        if sender() != get_current_miner() {
            return Err(errors::SENDER_IS_NOT_THE_WINNER);
        }
        let (bet_per_block, hash) = miners.get(&sender()).unwrap_or_abort();
        if !hash.to_vec().eq(&sha256(value.clone().to_vec())) {
            return Err(errors::INVALID_VALUE);
        }
        *miners.get_mut(&sender()).unwrap() = (*bet_per_block, value.clone());
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
        ellipticoin::get_storage::<_, Vec<u8>>(Namespace::RandomSeed as u8)[0..16]
            .try_into()
            .unwrap()
    }

    fn get_next_winner(miners: &HashMap<Vec<u8>, (u64, Vec<u8>)>) -> Vec<u8> {
        let random_seed: [u8; 16] = get_random_seed();
        let mut rng = SmallRng::from_seed(random_seed);
        let mut bets: Vec<(Vec<u8>, u64)> = miners
            .iter()
            .map(|(miner, (bet_per_block, _hash))| (miner.to_vec(), *bet_per_block))
            .collect();
        bets.sort_by(|(a, _), (b, _)| a.cmp(b));
        bets.choose_weighted(&mut rng, |(_miner, bet_per_block)| *bet_per_block)
            .map(|(miner, _bet_per_block)| miner.to_vec())
            .unwrap_or_abort()
    }

    fn get_miners() -> HashMap<Vec<u8>, (u64, Vec<u8>)> {
        from_value(ellipticoin::get_storage(Namespace::Miners as u8)).unwrap_or(HashMap::new())
    }

    fn set_miners(miners: &HashMap<Vec<u8>, (u64, Vec<u8>)>) {
        ellipticoin::set_storage(
            Namespace::Miners as u8,
            to_value(miners).unwrap_or(Value::Null),
        );
    }

    fn settle_block_rewards(winner: Vec<u8>, miners: &HashMap<Vec<u8>, (u64, Vec<u8>)>) {
        for (miner, (bet_per_block, _hash)) in miners {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipticoin::{set_block_number, set_sender};
    use ellipticoin_test_framework::{
        generate_hash_onion, random_bytes, sha256, ALICE, BOB, CAROL,
    };

    #[test]
    fn test_transfer() {
        set_sender(ALICE.to_vec());
        set_balance(ALICE.to_vec(), 100);
        transfer(BOB.to_vec(), 20).unwrap();
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 80);
        let bobs_balance = balance_of(BOB.to_vec());
        assert_eq!(bobs_balance, 20);
    }

    #[test]
    fn test_transfer_insufficient_funds() {
        set_sender(ALICE.to_vec());
        set_balance(ALICE.to_vec(), 100);
        assert!(transfer(BOB.to_vec(), 120).is_err());
    }

    #[test]
    fn test_transfer_from() {
        set_sender(ALICE.to_vec());
        set_balance(ALICE.to_vec(), 100);
        approve(BOB.to_vec(), 50);
        set_sender(BOB.to_vec());
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
    fn test_unlock_coins() {
        let ethereum_address = "43c01ab76d50c59e3893858ace624df81a14a596";
        set_sender(ALICE.to_vec());
        set_memory(
            Namespace::EthereumBalances,
            hex::decode(ethereum_address).unwrap(),
            1000 as u64,
        );
        unlock(hex::decode(&"4171741d3dbe24e2b4220b0be8be36b1f2dbc84be581137c43901fdb424ca8d22e325f518de61170706dac9dcd40eb9202f6623f234b22edd27cf4cdbd0eb7161c").unwrap());
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 1000);
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
        set_sender(ALICE.to_vec());
        start_mining(1, alices_onion.last().unwrap().to_vec()).unwrap();
        set_sender(BOB.to_vec());
        start_mining(1, bobs_onion.last().unwrap().to_vec()).unwrap();

        // With this random seed the winners are Alice, Alice, Bob in that order
        set_sender(ALICE.to_vec());
        alices_onion.pop();
        assert!(reveal(alices_onion.last().unwrap().to_vec()).is_ok());
        alices_onion.pop();
        assert!(reveal(alices_onion.last().unwrap().to_vec()).is_ok());
        set_sender(BOB.to_vec());
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
        set_sender(ALICE.to_vec());

        start_mining(1, hash).unwrap();
        set_block_number(1);
        assert!(reveal(invalid_value).is_err());
    }
}
