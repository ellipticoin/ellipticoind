pub mod constants;
pub mod errors;

use ellipticoin::{state_accessors, Address, Bytes, Token};
use errors::Error;
use std::convert::TryInto;
use wasm_rpc_macros::export_native;

pub const BASE_FACTOR: u64 = 1_000_000;
const CONTRACT_NAME: &'static str = "Token";

state_accessors!(
    balance(token: Token, address: Address) -> u64;
    total_supply(token: Token) -> u64;
);

export_native! {
    pub fn transfer<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        to: Bytes,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        debit(api, token.clone(), api.caller(), amount.clone())?;
        credit(api, token, to.try_into()?, amount);
        Ok(())
    }

    pub fn mint<API: ellipticoin::API>(
        api: &mut API,
        token_id: Bytes,
        address: Bytes,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        super::mint(
            api,
            Token {
                issuer: api.caller(),
                id: token_id,
            },
            address.try_into()?,
            amount,
        )
    }

    pub fn burn<API: ellipticoin::API>(
        api: &mut API,
        token_id: Bytes,
        address: Address,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        super::burn(
            api,
            Token {
                issuer: api.caller(),
                id: token_id,
            },
            address,
            amount,
        )
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

#[cfg(test)]
mod tests {
    use super::{native, *};
    use crate::system_contracts::test_api::TestAPI;
    use ellipticoin::Bytes;
    use std::{collections::HashMap, env};

    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB, CAROL};
    lazy_static! {
        static ref TOKEN: Token = Token {
            issuer: Address::PublicKey(*ALICE),
            id: vec![].into()
        };
    }
    #[test]
    fn test_transfer() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE), 100);
        native::transfer(&mut api, TOKEN.clone(), BOB.to_vec().into(), 20).unwrap();
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
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE), 100);
        assert!(
            native::transfer(&mut api, TOKEN.clone(), BOB.to_vec().into(), 120u8.into()).is_err()
        );
    }

    #[test]
    fn test_transfer_from_insufficient_funds() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = HashMap::new();
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
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(
            &mut api,
            TOKEN.clone().into(),
            Address::PublicKey(*ALICE),
            100,
        );
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
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        native::mint(&mut api, TOKEN.id.clone(), Bytes(ALICE.to_vec()), 50).unwrap();
        assert_eq!(
            get_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE)),
            50
        );
    }

    #[test]
    fn test_burn() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        set_balance(
            &mut api,
            TOKEN.clone().into(),
            Address::PublicKey(*ALICE),
            100,
        );
        set_total_supply(&mut api, TOKEN.clone().into(), 100);
        native::burn(&mut api, TOKEN.id.clone(), Address::PublicKey(*ALICE), 50).unwrap();
        assert_eq!(
            get_balance(&mut api, TOKEN.clone(), Address::PublicKey(*ALICE)),
            50
        );
        assert_eq!(get_total_supply(&mut api, TOKEN.clone()), 50);
    }
}
