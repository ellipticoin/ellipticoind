use ellipticoin::{
    block_number, block_winner, export, get_memory, sender, set_memory, Value
};
pub use wasm_rpc::{Bytes, Dereferenceable, Referenceable, FromBytes, ToBytes};

use issuance;
use errors;
use wasm_rpc::error::Error;

enum Namespace {
    Balances,
    Allowences,
    Mintings,
}

#[export]
mod system {
    pub fn constructor(initial_supply: u64) {
        set(Namespace::Balances, sender(), initial_supply)
    }

    pub fn approve(spender: Vec<u8>, amount: u64) {
        set(Namespace::Allowences, [sender(), spender].concat(), amount);
    }

    pub fn transfer_from(from: Vec<u8>, to: Vec<u8>, amount: u64) -> Result<Value, Error> {
        let allowance: u64 = get(Namespace::Allowences, [from.clone(), sender()].concat());

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
        if get::<_, u64>(Namespace::Balances, sender()) >= amount {
            debit(sender(), amount);
            credit(to, amount);
            Ok(Value::Null)
        } else {
            Err(errors::INSUFFICIENT_FUNDS)
        }
    }

    pub fn mint() -> Result<Value, Error> {
        if !block_minted(block_number()) {
            credit(block_winner(), block_reward(block_number()));
            mark_block_as_minted(block_number());
            Ok(Value::Null)
        } else {
            Err(errors::BLOCK_ALREADY_MINTED)
        }
    }

    fn block_minted(block_number: u64) -> bool {
        get(Namespace::Mintings, block_number)
    }

    fn block_reward(block_number: u64) -> u64 {
        issuance::block_reward(block_number)
    }

    fn debit_allowance(from: Vec<u8>, to: Vec<u8>, amount: u64) {
        let allowance: u64 = get(Namespace::Allowences, [from.clone(), to.clone()].concat());
        set(Namespace::Allowences, [from, to].concat(), allowance - amount);
    }

    fn mark_block_as_minted(block_number: u64) {
        set(Namespace::Mintings, block_number, true);
    }

    fn credit(address: Vec<u8>, amount: u64) {
        let balance: u64 = get(Namespace::Balances, address.clone());
        set(Namespace::Balances, address, balance + amount);
    }

    fn debit(address: Vec<u8>, amount: u64) {
        let balance: u64 = get(Namespace::Balances, address.clone());
        set(Namespace::Balances, address, balance - amount);
    }

    fn set<K: ToBytes, V: ToBytes>(namespace: Namespace, key: K, value: V) {
        set_memory([vec![namespace as u8], key.to_bytes()].concat(), value);
    }

    fn get<K: ToBytes, V: FromBytes>(namespace: Namespace, key: K) -> V {
        get_memory([vec![namespace as u8], key.to_bytes()].concat())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipticoin::{set_block_winner, set_sender};
    use ellipticoin_test_framework::{ALICE, BOB, CAROL};

    #[test]
    fn test_transfer() {
        set_sender(ALICE.to_vec());
        constructor(100);
        transfer(BOB.to_vec(), 20).unwrap();
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 80);
        let bobs_balance = balance_of(BOB.to_vec());
        assert_eq!(bobs_balance, 20);
    }

    #[test]
    fn test_transfer_insufficient_funds() {
        set_sender(ALICE.to_vec());
        constructor(100);
        assert!(transfer(BOB.to_vec(), 120).is_err());
    }

    #[test]
    fn test_transfer_from() {
        set_sender(ALICE.to_vec());
        constructor(100);
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

    #[test]
    fn test_mint() {
        set_sender(ALICE.to_vec());
        set_block_winner(ALICE.to_vec());
        constructor(100);
        mint().expect("failed to mint");
        let alices_balance = balance_of(ALICE.to_vec());
        assert_eq!(alices_balance, 640100);
    }

    #[test]
    fn test_block_cannot_be_minted_twice() {
        set_sender(ALICE.to_vec());
        set_block_winner(ALICE.to_vec());
        constructor(100);
        mint().expect("failed to mint");
        assert!(mint().is_err());
    }

    pub fn balance_of(address: Vec<u8>) -> u64 {
        get(Namespace::Balances, address)
    }

    pub fn allowance(owner: Vec<u8>, spender: Vec<u8>) -> u64 {
        get(Namespace::Allowences, [owner, spender].concat())
    }
}
