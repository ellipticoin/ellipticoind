mod errors;

use crate::helpers::sha256;
use ellipticoin::Address;
use errors::Error;
use num_traits::cast::ToPrimitive;
use serde_cbor::Value;
use wasm_rpc_macros::export_native;

enum Namespace {
    Allowances,
    Balances,
}

export_native! {
    pub fn mint<API: ellipticoin::API>(
        api: &mut API,
        token_id: [u8; 32],
        to: Address,
        amount: u64
    ) -> Result<Value, Box<Error>> {
        credit(api, api.caller(), token_id.clone(), to, amount).unwrap();
        Ok(serde_cbor::Value::Null)
    }

    pub fn transfer<API: ellipticoin::API>(
        api: &mut API,
        issuer: Address,
        token_id: [u8; 32],
        to: Address,
        amount: u64
    ) -> Result<Value, Box<Error>> {
        if get_balance(api, issuer.clone(), token_id.clone(), api.caller()) >= amount {
            debit(api, issuer.clone(), token_id.clone(), api.caller(), amount.clone()).unwrap();
            credit(api, issuer.clone(), token_id.clone(), to, amount).unwrap();
            Ok(get_balance(api, issuer, token_id, api.caller()).to_u32().unwrap().into())
        } else {
            Err(Box::new(errors::INSUFFICIENT_FUNDS.clone()))
        }
    }

    pub fn approve<API: ellipticoin::API>(
        api: &mut API,
        issuer: Address,
        token_id: [u8; 32],
        spender: Address,
        amount: u64
    ) -> Result<Value, Box<Error>> {
        set_allowance(api, issuer, token_id,api.caller(), spender, amount);
        Ok(serde_cbor::Value::Null)
    }

    pub fn transfer_from<API: ellipticoin::API>(
        api: &mut API,
        issuer: Address,
        token_id: [u8; 32],
        from: Address,
        to: Address,
        amount: u64
    ) -> Result<Value, Box<Error>> {
        if get_allowance(api, issuer.clone(), token_id.clone(), from.clone(), api.caller()) < amount {
            return Err(Box::new(errors::INSUFFICIENT_ALLOWANCE.clone()));
        }

        if get_balance(api, issuer.clone(), token_id.clone(), from.clone()) < amount {
            return Err(Box::new(errors::INSUFFICIENT_FUNDS.clone()));
        }

        debit_allowance(api, issuer.clone(), token_id.clone(), from.clone(), api.caller(), amount.clone());
        debit(api, issuer.clone(), token_id.clone(), from, amount.clone()).unwrap();
        credit(api, issuer, token_id, to, amount).unwrap();
        Ok(Value::Null)
    }

    fn debit_allowance<API: ellipticoin::API>(
        api: &mut API,
        issuer: Address,
        token_id: [u8; 32],
        from: Address,
        to: Address,
        amount: u64
    ) {
        let allowance = get_allowance(api, issuer.clone(), token_id.clone(), from.clone(), to.clone());
        set_allowance(api, issuer, token_id, from, to, allowance - amount);
    }

    pub fn get_balance<API: ellipticoin::API>(
        api: &mut API,
        mut issuer: Address,
        token_id: [u8; 32],
        mut address: Address
    ) -> u64 {
        api.get_memory(&[
            &[
                Namespace::Balances as u8].to_vec(),
                &sha256(issuer.to_vec())[..],
                &token_id,
                &address.to_vec()[..]
            ].concat()
        ).unwrap_or(0u8.into())
    }

    fn get_allowance<API: ellipticoin::API>(
        api: &mut API,
        mut issuer: Address,
        token_id: [u8; 32],
        mut from: Address,
        mut to: Address
    ) -> u64 {
        api.get_memory(&[
            &[Namespace::Allowances as u8],
            &sha256(issuer.to_vec())[..],
            &token_id,
            &from.to_vec()[..],
            &to.to_vec()[..]
            ].concat(),
        ).unwrap_or(0u8.into())
    }

    fn set_allowance<API: ellipticoin::API>(
        api: &mut API,
        mut issuer: Address,
        token_id: [u8; 32],
        mut from: Address,
        mut to: Address,
        value: u64
    ) {
        api.set_memory(&[
            [Namespace::Allowances as u8].to_vec(),
            sha256(issuer.to_vec()).to_vec(),
            token_id.to_vec(),
            from.to_vec(), to.to_vec()].concat(),
            value,
        );
    }

    pub fn credit<API: ellipticoin::API>(
        api: &mut API,
        mut issuer: Address,
        token_id: [u8; 32],
        mut address: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        check_caller(api, &issuer)?;
        let balance: u64 = get_balance(api, issuer.clone(), token_id, address.clone());
        api.set_memory(
&[
            &[
                Namespace::Balances as u8][..],
            &sha256(issuer.to_vec())[..],
            &token_id,
            &address.to_vec()[..]
].concat()
, balance + amount);
        Ok(())
    }

    pub fn debit<API: ellipticoin::API>(
        api: &mut API,
        mut issuer: Address,
        token_id: [u8; 32],
        mut address: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        check_caller(api, &issuer)?;
        let balance: u64 = get_balance(api, issuer.clone(), token_id, address.clone());
        api.set_memory(&[
            &[
                Namespace::Balances as u8][..],
            &sha256(issuer.to_vec())[..],
            &token_id,
            &address.to_vec()[..]
].concat(), balance - amount);
        Ok(())
    }

    fn check_caller<API: ellipticoin::API>(
        api: &mut API,
        caller: &Address
) -> Result<(), Box<Error>> {
        if api.caller() == *caller || api.caller() == ellipticoin::Address::PublicKey(api.sender()) {
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
    use crate::system_contracts::api::{TestAPI, TestState};
    use ellipticoin::{constants::SYSTEM_ADDRESS, API};
    use std::env;

    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB, CAROL};

    #[test]
    fn test_transfer() {
        let token_id = [0; 32];
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        set_balance(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id.clone(),
            *ALICE,
            100,
        );
        let result = transfer(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id.clone(),
            Address::PublicKey(*BOB),
            20,
        )
        .unwrap();
        assert_eq!(result, Value::Integer(80u8.into()));
        assert_eq!(
            get_balance(
                &mut api,
                Address::PublicKey(*ALICE),
                token_id.clone(),
                Address::PublicKey(*ALICE)
            ),
            80
        );
        assert_eq!(
            get_balance(
                &mut api,
                Address::PublicKey(*ALICE),
                token_id,
                Address::PublicKey(*BOB)
            ),
            20
        );
    }
    #[test]
    fn test_transfer_insufficient_funds() {
        let token_id = [0; 32];
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        set_balance(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id.clone(),
            *ALICE,
            100,
        );
        assert!(transfer(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id,
            Address::PublicKey(*BOB),
            120u8.into()
        )
        .is_err());
    }

    #[test]
    fn test_transfer_from_insufficient_funds() {
        let token_id = [0; 32];
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        set_balance(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id.clone(),
            *BOB,
            100,
        );
        assert!(transfer_from(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id,
            Address::PublicKey(*BOB),
            Address::PublicKey(*CAROL),
            120
        )
        .is_err());
    }

    #[test]
    fn test_transfer_from() {
        let token_id = [0; 32];
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        set_balance(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id.clone(),
            *ALICE,
            100,
        );
        approve(
            &mut api,
            ellipticoin::Address::PublicKey(*ALICE),
            token_id.clone(),
            ellipticoin::Address::PublicKey(*BOB),
            50,
        )
        .unwrap();
        api.sender = *BOB;
        api.caller = Address::PublicKey(*BOB);
        transfer_from(
            &mut api,
            Address::PublicKey(*ALICE),
            token_id.clone(),
            Address::PublicKey(*ALICE),
            Address::PublicKey(*CAROL),
            20,
        )
        .unwrap();
        assert_eq!(
            get_balance(
                &mut api,
                Address::PublicKey(*ALICE),
                token_id.clone(),
                Address::PublicKey(*ALICE)
            ),
            80
        );
        assert_eq!(
            get_balance(
                &mut api,
                Address::PublicKey(*ALICE),
                token_id,
                Address::PublicKey(*CAROL)
            ),
            20
        );
    }

    pub fn set_balance(
        api: &mut TestAPI,
        mut issuer: Address,
        token_id: [u8; 32],
        address: [u8; 32],
        balance: u64,
    ) {
        api.set_memory(
            &[
                &[Namespace::Balances as u8][..],
                &sha256(issuer.to_vec())[..],
                &token_id,
                &address[..],
            ]
            .concat(),
            balance,
        );
    }
}
