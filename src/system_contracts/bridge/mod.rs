pub mod constants;
mod errors;

use crate::system_contracts::token::{self};
use constants::SIGNERS;
use ellipticoin::{Address, Token};
use std::boxed::Box;
use wasm_rpc::error::Error;
use wasm_rpc_macros::export_native;

const CONTRACT_NAME: &'static str = "Bridge";

export_native! {
    pub fn mint<API: ellipticoin::API>(
        api: &mut API,
        token_id: [u8; 32],
        address: Address,
        amount: u64,
    ) -> Result<(), Box<Error>> {
    if SIGNERS
        .iter()
        .any(|&signer| Address::PublicKey(signer) == api.caller())
    {
        token::mint(api, token(token_id), address, amount)?;
    } else {
        return Err(Box::new(errors::INVALID_SIGNER.clone()));
    }
        Ok(())
   }

    pub fn release<API: ellipticoin::API>(
        api: &mut API,
        token_id: [u8; 32],
        _address: [u8; 20],
        amount: u64,
    ) -> Result<(), Box<Error>> {
        token::burn(api, token(token_id),api.caller(), amount)?;
        Ok(())
   }
}

pub fn token(token_id: [u8; 32]) -> Token {
    Token {
        issuer: Address::Contract(CONTRACT_NAME.to_string()),
        id: token_id,
    }
}

#[cfg(test)]
mod tests {
    use super::{native, *};
    use crate::system_contracts::{
        test_api::{TestAPI, TestState},
        token,
        token::BASE_FACTOR,
    };
    use ellipticoin::constants::{ELC, SYSTEM_ADDRESS};
    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB};
    use std::env;
    const BTC: [u8; 32] = [0; 32];
    const ETH_ADDRESS: [u8; 20] = [0; 20];

    #[test]
    fn test_mint() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(
            &mut state,
            SIGNERS[0],
            "Token".to_string(),
        );
        native::mint(&mut api, BTC, Address::PublicKey(*ALICE), 1 * BASE_FACTOR).unwrap();
        assert_eq!(
            token::get_balance(&mut api, token(BTC.clone()), Address::PublicKey(*ALICE)),
            1 * BASE_FACTOR
        );
    }

    #[test]
    fn test_mint_invalid_sender() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        assert!(native::mint(&mut api, BTC, Address::PublicKey(*ALICE), 1 * BASE_FACTOR).is_err());
    }

    #[test]
    fn test_release() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(
            &mut state,
            SIGNERS[0],
            "Token".to_string(),
        );
        native::mint(&mut api, BTC, Address::PublicKey(*ALICE), 1 * BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(ALICE.clone());
        native::release(&mut api, BTC, ETH_ADDRESS, 1 * BASE_FACTOR).unwrap();
        assert_eq!(
            token::get_balance(&mut api, token(BTC.clone()), Address::PublicKey(*ALICE)),
            0 * BASE_FACTOR
        );
    }
}
