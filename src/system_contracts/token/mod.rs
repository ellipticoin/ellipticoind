mod errors;

use ellipticoin::{helpers::is_system_address, memory_accessors, Address, Token};
use errors::Error;
use serde_cbor::Value;
use wasm_rpc_macros::export_native;

pub const BASE_FACTOR: u64 = 1_000_000;

memory_accessors!(allowance: u64, balance: u64);

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
        debit(api, token.clone(), api.caller(), amount.clone())?;
        Ok(credit(api, token.clone(), to, amount).unwrap())
    }

    pub fn approve<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut spender: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        set_allowance(api, [token.into(),api.caller().to_vec(), spender.to_vec()].concat(), amount);
        Ok(())
    }

    pub fn transfer_from<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        sender: Address,
        recipient: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        debit_allowance(api, token.clone(), sender.clone(), api.caller(), amount.clone())?;
        debit(api, token.clone(), sender, amount.clone())?;
        credit(api, token.clone(), recipient.clone(), amount).unwrap();
        Ok(())
    }


    pub fn credit<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut address: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        check_caller(api, &token.issuer)?;
        let balance = get_balance(api, [token.clone().into(), address.to_vec()].concat());
        Ok(set_balance(api, [token.clone().into(), address.to_vec()].concat(), balance + amount))
    }

    pub fn debit<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        mut address: Address,
        amount: u64
    ) -> Result<(), Box<Error>> {
        check_caller(api, &token.issuer)?;
        let balance = get_balance(api, [token.clone().into(), address.to_vec()].concat());
        if amount <= balance {

        Ok(set_balance(api, [token.clone().into(), address.to_vec()].concat(), balance - amount))
        } else {
            Err(Box::new(errors::INSUFFICIENT_FUNDS.clone()))
        }
    }

    fn check_caller<API: ellipticoin::API>(
        api: &mut API,
        caller: &Address
) -> Result<(), Box<Error>> {
        if api.caller() == *caller || api.caller() == ellipticoin::Address::PublicKey(api.sender()) || is_system_address(api.caller()) {
            Ok(())
        } else {
            return Err(Box::new(wasm_rpc::error::Error{
                code: 3,
                message: "Invalid caller".to_string(),
            }));
        }
    }
}

fn debit_allowance<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    mut owner: Address,
    mut spender: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    if is_system_address(spender.clone()) {
        return Ok(());
    }

    let allowance = get_allowance(
        api,
        [
            token.clone().into(),
            owner.clone().to_vec(),
            spender.clone().to_vec(),
        ]
        .concat(),
    );
    if amount <= allowance {
        Ok(set_allowance(
            api,
            [token.into(), owner.to_vec(), spender.to_vec()].concat(),
            allowance - amount,
        ))
    } else {
        Err(Box::new(errors::INSUFFICIENT_ALLOWANCE.clone()))
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
            get_balance(
                &mut api,
                [TOKEN.clone().into(), Address::PublicKey(*ALICE).to_vec()].concat()
            ),
            80
        );
        assert_eq!(
            get_balance(
                &mut api,
                [TOKEN.clone().into(), Address::PublicKey(*BOB).to_vec()].concat()
            ),
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
            get_balance(
                &mut api,
                [TOKEN.clone().into(), Address::PublicKey(*ALICE).to_vec()].concat()
            ),
            80
        );
        assert_eq!(
            get_balance(
                &mut api,
                [TOKEN.clone().into(), Address::PublicKey(*CAROL).to_vec()].concat()
            ),
            20
        );
    }
}
