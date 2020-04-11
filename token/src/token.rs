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
    Balances,
    Allowences,
    SpoonedBalances,
    Miners,
}

#[export]
mod token {
    pub fn approve(spender: Vec<u8>, amount: u64) {
        set_memory(Namespace::Allowences, [sender(), spender].concat(), amount);
    }

    pub fn transfer_from(from: Vec<u8>, to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        let allowance: u64 = get_memory(Namespace::Allowences, [from.clone(), sender()].concat());

        if allowance >= amount {
            debit_allowance(from.clone(), sender(), amount);
            debit(from, amount);
            credit(to, amount);
            Ok(Value::Null)
        } else {
            Err(errors::INSUFFICIENT_FUNDS)
        }
    }

    pub fn transfer(to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        if get_memory::<_, u64>(Namespace::Balances, sender()) >= amount {
            debit(sender(), amount);
            credit(to, amount);
            Ok(Value::Null)
        } else {
            Err(errors::INSUFFICIENT_FUNDS)
        }
    }

    fn debit_allowance(from: Vec<u8>, to: Vec<u8>, amount: u64) {
        let allowance: u64 = get_memory(Namespace::Allowences, [from.clone(), to.clone()].concat());
        set_memory(
            Namespace::Allowences,
            [from, to].concat(),
            allowance - amount,
        );
    }

    pub fn unlock(unlock_signature: Vec<u8>) {
        let message = "unlock_ellipticoin";
        let address = ethereum::ecrecover_address(message.as_bytes(), &unlock_signature);
        let balance = get_memory(Namespace::SpoonedBalances, address);
        credit(sender(), balance);
    }

    pub fn start_mining(bet_per_block: u64, hash_onion: Vec<u8>) -> Result<Value, Error> {
        let mut miners = get_miners();
        miners.insert(sender(), (bet_per_block, hash_onion.clone()));
        set_miners(miners);
        Ok(Value::Null)
    }

    pub fn reveal(value: Vec<u8>) -> Result<Value, Error> {
        let miners = get_miners();
        let hash = match miners.get(&sender()) {
            Some((_bet_per_block, hash)) => hash,
            None => return Err(errors::NOT_CURRENTLY_MINING),
        };
        if !hash.to_vec().eq(&sha256(value)) {
            return Err(errors::INVALID_VALUE);
        }
        settle_block_rewards(sender(), miners);
        Ok(Value::Null)
    }

    fn get_miners() -> HashMap<Vec<u8>, (u64, Vec<u8>)> {
        from_value(ellipticoin::get_memory(Namespace::Miners as u8)).unwrap_or(HashMap::new())
    }

    fn set_miners(miners: HashMap<Vec<u8>, (u64, Vec<u8>)>) {
        ellipticoin::set_memory(
            Namespace::Miners as u8,
            to_value(miners).unwrap_or(Value::Null),
        );
    }

    fn settle_block_rewards(winner: Vec<u8>, miners: HashMap<Vec<u8>, (u64, Vec<u8>)>) {
        for (miner, (bet_per_block, _hash)) in miners {
            if miner == winner {
                credit(miner, bet_per_block);
            } else {
                debit(miner, bet_per_block);
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
            Namespace::SpoonedBalances,
            hex::decode(ethereum_address).unwrap(),
            1000 as u64,
        );
        unlock(hex::decode(&"4171741d3dbe24e2b4220b0be8be36b1f2dbc84be581137c43901fdb424ca8d22e325f518de61170706dac9dcd40eb9202f6623f234b22edd27cf4cdbd0eb7161c").unwrap());
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 1000);
    }

    #[test]
    fn test_commit_and_reveal() {
        set_balance(ALICE.to_vec(), 1);
        set_balance(BOB.to_vec(), 1);
        let alices_center = [0; 32].to_vec();
        let bobs_center = [1; 32].to_vec();
        let mut alices_onion = generate_hash_onion(3, alices_center.clone());
        let mut bobs_onion = generate_hash_onion(3, bobs_center.clone());
        set_sender(ALICE.to_vec());
        start_mining(1, alices_onion.last().unwrap().to_vec()).unwrap();
        set_sender(BOB.to_vec());
        start_mining(1, bobs_onion.last().unwrap().to_vec()).unwrap();
        alices_onion.pop();
        bobs_onion.pop();

        set_block_number(1);
        set_sender(ALICE.to_vec());
        assert!(reveal(alices_onion.last().unwrap().to_vec()).is_ok());
        assert_eq!(balance_of(ALICE.to_vec()), 2);
        assert_eq!(balance_of(BOB.to_vec()), 0);
    }

    #[test]
    fn test_commit_and_reveal_invalid() {
        let value = random_bytes(32);
        let hash = sha256(value.clone());
        let invalid_value = random_bytes(32);
        set_sender(ALICE.to_vec());

        start_mining(1, hash).unwrap();
        set_block_number(1);
        assert!(reveal(invalid_value).is_err());
    }
}
