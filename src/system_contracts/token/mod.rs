pub mod constants;
mod errors;

use ellipticoin::{memory_accessors, Address, Token};
use errors::Error;
use wasm_rpc_macros::export_native;

pub const BASE_FACTOR: u64 = 1_000_000;
const CONTRACT_NAME: &'static str = "Token";

memory_accessors!(
    allowance(token: Token, address: Address, spender: Address) -> u64;
    balance(token: Token, address: Address) -> u64;
    total_supply(token: Token) -> u64;
);

export_native! {
pub fn transfer<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    to: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    debit(api, token.clone(), api.caller(), amount.clone())?;
    credit(api, token, to, amount);
    Ok(())
}

pub fn approve<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    spender: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    set_allowance(api, token, api.caller(), spender, amount);
    Ok(())
}

pub fn mint<API: ellipticoin::API>(
    api: &mut API,
    token_id: [u8; 32],
    address: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    super::mint(
        api,
        Token {
            issuer: api.caller(),
            id: token_id,
        },
        address,
        amount,
    )
}

pub fn transfer_from<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    sender: Address,
    recipient: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    debit_allowance(
        api,
        token.clone(),
        sender.clone(),
        api.caller(),
        amount.clone(),
    )?;
    transfer_from(api, token, sender, recipient, amount)?;
    Ok(())
}

}
pub fn transfer_from<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    sender: Address,
    recipient: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    debit(api, token.clone(), sender, amount.clone())?;
    credit(api, token.clone(), recipient.clone(), amount);
    Ok(())
}

pub fn mint<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    to: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    credit(api, token.clone(), to, amount);
    let total_supply = get_total_supply(api, token.clone());
    set_total_supply(api, token, total_supply + amount);
    Ok(())
}

pub fn burn<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    to: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    debit(api, token.clone(), to, amount)?;
    let total_supply = get_total_supply(api, token.clone());
    set_total_supply(api, token, total_supply - amount);
    Ok(())
}

pub fn credit<API: ellipticoin::API>(api: &mut API, token: Token, address: Address, amount: u64) {
    let balance = get_balance(api, token.clone(), address.clone());
    set_balance(api, token.clone(), address, balance + amount)
}

pub fn debit<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    address: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    let balance = get_balance(api, token.clone(), address.clone());
    if amount <= balance {
        Ok(set_balance(api, token.clone(), address, balance - amount))
    } else {
        Err(Box::new(errors::INSUFFICIENT_FUNDS.clone()))
    }
}

fn debit_allowance<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    owner: Address,
    spender: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    let allowance = get_allowance(api, token.clone(), owner.clone(), spender.clone());
    if amount <= allowance {
        Ok(set_allowance(
            api,
            token,
            owner,
            spender,
            allowance - amount,
        ))
    } else {
        Err(Box::new(errors::INSUFFICIENT_ALLOWANCE.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{native, *};
    use crate::system_contracts::test_api::{TestAPI, TestState};
    use std::env;

    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB, CAROL};
    lazy_static! {
        static ref TOKEN: Token = Token {
            issuer: Address::PublicKey(*ALICE),
            id: [0; 32]
        };
    }
    #[test]
    fn test_transfer() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE), 100);
        native::transfer(&mut api, TOKEN.clone(), Address::PublicKey(*BOB), 20).unwrap();
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
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE), 100);
        assert!(native::transfer(
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
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(&mut api, TOKEN.clone(), Address::PublicKey(*BOB), 100);
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
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(
            &mut api,
            TOKEN.clone().into(),
            Address::PublicKey(*ALICE),
            100,
        );
        native::approve(
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
    #[test]
    fn test_mint() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        native::mint(&mut api, TOKEN.id, Address::PublicKey(*ALICE), 50).unwrap();
        assert_eq!(
            get_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE)),
            50
        );
    }
}
