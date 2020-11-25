pub mod constants;
mod errors;

use crate::system_contracts::token::{self};
use constants::SIGNERS;
use ellipticoin::{Address, Bytes, Token};
use std::{boxed::Box, convert::TryInto};
use wasm_rpc::error::Error;
use wasm_rpc_macros::export_native;

const CONTRACT_NAME: &'static str = "Bridge";

export_native! {
    pub fn mint<API: ellipticoin::API>(
        api: &mut API,
        token_id: Bytes,
        address: Bytes,
        amount: u64,
    ) -> Result<(), Box<Error>> {
    if SIGNERS
        .iter()
        .any(|&signer| Address::PublicKey(signer) == api.caller())
    {
        token::mint(api, token(token_id), address.try_into()?, amount)?;
    } else {
        return Err(Box::new(errors::INVALID_SIGNER.clone()));
    }
        Ok(())
   }

    pub fn release<API: ellipticoin::API>(
        api: &mut API,
        token_id: Bytes,
        _address: Bytes,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        token::burn(api, token(token_id),api.caller(), amount)?;
        Ok(())
   }
}

pub fn token(token_id: Bytes) -> Token {
    Token {
        issuer: Address::Contract(CONTRACT_NAME.to_string()),
        id: token_id,
    }
}

#[cfg(test)]
mod tests {
    use super::{native, *};
    use crate::system_contracts::{test_api::TestAPI, token, token::BASE_FACTOR};
    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY};
    use std::{collections::HashMap, env};

    const BTC: [u8; 20] = [0; 20];
    const ETH_ADDRESS: [u8; 20] = [0; 20];

    #[test]
    fn test_mint() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, SIGNERS[0], "Token".to_string());
        native::mint(
            &mut api,
            BTC.to_vec().into(),
            Bytes(ALICE.to_vec()),
            1 * BASE_FACTOR,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                token(BTC.to_vec().into()),
                Address::PublicKey(*ALICE)
            ),
            1 * BASE_FACTOR
        );
    }

    #[test]
    fn test_mint_invalid_sender() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, *ALICE, "Token".to_string());
        assert!(native::mint(
            &mut api,
            BTC.to_vec().into(),
            Bytes(ALICE.to_vec()),
            1 * BASE_FACTOR
        )
        .is_err());
    }

    #[test]
    fn test_release() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = HashMap::new();
        let mut api = TestAPI::new(&mut state, SIGNERS[0], "Token".to_string());
        native::mint(
            &mut api,
            BTC.to_vec().into(),
            Bytes::from(ALICE.to_vec()),
            1 * BASE_FACTOR,
        )
        .unwrap();
        api.caller = Address::PublicKey(ALICE.clone());
        native::release(
            &mut api,
            BTC.to_vec().into(),
            Bytes(ETH_ADDRESS.to_vec()),
            1 * BASE_FACTOR,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                token(BTC.to_vec().into()),
                Address::PublicKey(*ALICE)
            ),
            0 * BASE_FACTOR
        );
    }
}
