mod errors;

use ellipticoin::{constants::SYSTEM_ADDRESS, Address, Token};
use errors::Error;
use serde_cbor::Value;
use wasm_rpc_macros::export_native;

pub enum Namespace {
    Allowances,
    Balances,
}

pub const BASE_FACTOR: u64 = 1_000_000;

export_native! {
    pub fn mint<API: ellipticoin::API>(
        api: &mut API,
        token_id: [u8; 32],
        to: Address,
        amount: u64
    ) -> Result<Value, Box<Error>> {
        let token = Token {
            issuer: api.caller(),
            token_id,
        };
        credit(api, token, to, amount).unwrap();
        Ok(serde_cbor::Value::Null)
    }

    pub fn transfer<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        to: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        if get_balance(api, token.clone(), api.caller()) >= amount {
            debit(api, token.clone(), api.caller(), amount.clone()).unwrap();
            credit(api, token.clone(), to, amount).unwrap();
            Ok(())
        } else {
            Err(Box::new(errors::INSUFFICIENT_FUNDS.clone()))
        }
    }

    pub fn approve<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        spender: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        set_allowance(api, token,api.caller(), spender, amount);
        Ok(())
    }

    pub fn transfer_from<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        from: Address,
        to: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        if get_allowance(api, token.clone(), from.clone(), api.caller()) < amount {
            return Err(Box::new(errors::INSUFFICIENT_ALLOWANCE.clone()));
        }

        let balance = get_balance(api, token.clone(), from.clone());
        if balance < amount {
            return Err(Box::new(errors::INSUFFICIENT_FUNDS.clone()));
        }

        debit_allowance(api, token.clone(), from.clone(), api.caller(), amount.clone());
        debit(api, token.clone(), from, amount.clone()).unwrap();
        credit(api, token.clone(), to.clone(), amount).unwrap();
        Ok(())
    }

    fn debit_allowance<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        from: Address,
        to: Address,
        amount: u64
    ) {
        let allowance = get_allowance(api, token.clone(), from.clone(), to.clone());
        set_allowance(api, token, from, to, allowance - amount);
    }

    pub fn get_balance<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut address: Address
    ) -> u64 {
        api.get_memory([
            [
                Namespace::Balances as u8].to_vec(),
                token.into(),
                address.to_vec()
            ].concat()
        ).unwrap_or(0u8.into())
    }

    fn get_allowance<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut from: Address,
        mut to: Address
    ) -> u64 {
        #[allow(non_snake_case)]
        if matches!(to, Address::Contract((_SYSTEM_ADDRESS,_))) {
            return u64::MAX
        }else {
};

        api.get_memory([
            [Namespace::Allowances as u8].to_vec(),
            token.into(),
            from.to_vec(),
            to.to_vec()
            ].concat(),
        ).unwrap_or(0u8.into())
    }

    fn set_allowance<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut from: Address,
        mut to: Address,
        value: u64
    ) {
        api.set_memory([
            [Namespace::Allowances as u8].to_vec(),
            token.into(),
            from.to_vec(), to.to_vec()].concat(),
            value,
        );
    }

    pub fn credit<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut address: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        check_caller(api, &token.issuer)?;
        let balance: u64 = get_balance(api, token.clone(), address.clone());
        api.set_memory(
[
            [
                Namespace::Balances as u8].to_vec(),
            token.into(),
            address.to_vec()
].concat()
, balance + amount);
        Ok(())
    }

    pub fn debit<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut address: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        check_caller(api, &token.issuer)?;
        let balance: u64 = get_balance(api, token.clone(), address.clone());
        api.set_memory([
            [
                Namespace::Balances as u8].to_vec(),
            token.into(),
            address.to_vec()
].concat(), balance - amount);
        Ok(())
    }

    fn check_caller<API: ellipticoin::API>(
        api: &mut API,
        caller: &Address
) -> Result<(), Box<Error>> {
        if api.caller() == *caller || api.caller() == ellipticoin::Address::PublicKey(api.sender()) || matches!(api.caller(), Address::Contract((SYSTEM_ADDRESS, _))) {
            Ok(())
        } else {
            return Err(Box::new(wasm_rpc::error::Error{
                code: 3,
                message: "Invalid caller".to_string(),
            }));
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_contracts::test_api::{TestAPI, TestState};
    use ellipticoin::constants::SYSTEM_ADDRESS;
    use std::env;

    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB, CAROL};
    lazy_static! {
        static ref TOKEN: Token = Token {
            issuer: Address::PublicKey(*ALICE),
            token_id: [0; 32]
        };
    }
    #[test]
    fn test_transfer() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        api.set_balance(TOKEN.clone(), *ALICE, 100);
        transfer(&mut api, TOKEN.clone(), Address::PublicKey(*BOB), 20).unwrap();
        assert_eq!(
            get_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE)),
            80
        );
        assert_eq!(
            get_balance(&mut api, TOKEN.clone(), Address::PublicKey(*BOB)),
            20
        );
    }
    #[test]
    fn test_transfer_insufficient_funds() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        api.set_balance(TOKEN.clone(), *ALICE, 100);
        assert!(transfer(
            &mut api,
            TOKEN.clone(),
            Address::PublicKey(*BOB),
            120u8.into()
        )
        .is_err());
    }

    #[test]
    fn test_transfer_from_insufficient_funds() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        api.set_balance(TOKEN.clone(), *BOB, 100);
        assert!(transfer_from(
            &mut api,
            TOKEN.clone(),
            Address::PublicKey(*BOB),
            Address::PublicKey(*CAROL),
            120
        )
        .is_err());
    }

    #[test]
    fn test_transfer_from() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        api.set_balance(TOKEN.clone(), *ALICE, 100);
        approve(
            &mut api,
            TOKEN.clone(),
            ellipticoin::Address::PublicKey(*BOB),
            50,
        )
        .unwrap();
        api.sender = *BOB;
        api.caller = Address::PublicKey(*BOB);
        transfer_from(
            &mut api,
            TOKEN.clone(),
            Address::PublicKey(*ALICE),
            Address::PublicKey(*CAROL),
            20,
        )
        .unwrap();
        assert_eq!(
            get_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE)),
            80
        );
        assert_eq!(
            get_balance(&mut api, TOKEN.clone(), Address::PublicKey(*CAROL)),
            20
        );
    }
}
