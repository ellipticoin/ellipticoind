mod constants;
mod errors;

use crate::{
    helpers::sha256,
    system_contracts::token::{self, BASE_FACTOR},
};
use constants::{BASE_TOKEN, FEE};
use ellipticoin::{constants::SYSTEM_ADDRESS, memory_accessors, Address, Token};
use std::boxed::Box;
use wasm_rpc::error::Error;
use wasm_rpc_macros::export_native;

const CONTRACT_NAME: &'static str = "Exchange";

memory_accessors!(
    base_token_reserves(token: Token) -> u64;
);

export_native! {
    pub fn create_pool<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
        starting_price: u64,
    ) -> Result<(), Box<Error>> {
        charge(api, token.clone(), api.caller(), amount)?;
        charge(api, BASE_TOKEN.clone(), api.caller(), (amount * starting_price) / BASE_FACTOR)?;
        credit_base_token_reserves(api, token.clone(), (amount * starting_price) / BASE_FACTOR);
        token::mint(api, pool_token(token), api.caller(), amount)?;
        Ok(())
    }

    pub fn add_liqidity<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        let ratio = get_ratio(api, token.clone())?;
        charge(api, token.clone(), api.caller(), amount)?;
        charge(api, BASE_TOKEN.clone(), api.caller(), amount * (ratio/BASE_FACTOR))?;
        credit_base_token_reserves(api, token.clone(), amount * (ratio/BASE_FACTOR));
        token::mint(api, pool_token(token), api.caller(), amount)?;
        Ok(())
    }

    pub fn remove_liqidity<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        let reserves = get_reserves(api, token.clone());
        let base_token_reserves = get_base_token_reserves(api, token.clone());
        let total_supply = token::get_total_supply(api, pool_token(token.clone()));
        debit_base_token_reserves(api, token.clone(), (base_token_reserves * amount) / total_supply);
        pay(api, token.clone(), api.caller(), (reserves * amount) / total_supply)?;
        pay(api, BASE_TOKEN.clone(), api.caller(), (base_token_reserves * amount) / total_supply)?;
        token::burn(api, pool_token(token), api.caller(), amount)?;
        Ok(())
    }

    pub fn swap<API: ellipticoin::API>(
        api: &mut API,
        input_token: Token,
        output_token: Token,
        input_amount: u64,
    ) -> Result<(), Box<Error>> {
        let base_token_amount = if input_token == BASE_TOKEN.clone() {
            input_amount
        } else {
            rebalance_base_token(api, input_token.clone(), apply_fee(input_amount))?
        };
        charge(api, input_token.clone(), api.caller(), input_amount)?;
        let token_amount = if output_token == BASE_TOKEN.clone() {
            base_token_amount
        } else {
            let token_amount = rebalance(api, output_token.clone(), apply_fee(base_token_amount))?;
            credit_base_token_reserves(api, output_token.clone(), base_token_amount);
            token_amount
        };
        pay(api, output_token.clone(), api.caller(), token_amount)?;
        Ok(())
    }
}

fn rebalance_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<u64, Box<Error>> {
    let total_value = get_total_value(api, token.clone());
    let new_base_token_reserves = total_value / (get_reserves(api, token.clone()) + amount);
    Ok(get_base_token_reserves(api, token.clone()) - new_base_token_reserves)
}

fn rebalance<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<u64, Box<Error>> {
    let total_value = get_total_value(api, token.clone());
    credit_base_token_reserves(api, token.clone(), amount);
    let new_token_reserves = total_value / get_base_token_reserves(api, token.clone());
    Ok(get_reserves(api, token.clone()) - new_token_reserves)
}

fn apply_fee(amount: u64) -> u64 {
    amount - ((amount * FEE) / BASE_FACTOR)
}

fn get_reserves<API: ellipticoin::API>(api: &mut API, token: Token) -> u64 {
    token::get_balance(
        api,
        token,
        Address::Contract((SYSTEM_ADDRESS, CONTRACT_NAME.to_string())),
    )
}

fn get_ratio<API: ellipticoin::API>(api: &mut API, token: Token) -> Result<u64, Box<Error>> {
    let base_token_reserves = get_base_token_reserves(api, token.clone());
    if base_token_reserves == 0 {
        Err(Box::new(errors::POOL_NOT_FOUND.clone()))
    } else {
        Ok(token::get_balance(
            api,
            token,
            Address::Contract((SYSTEM_ADDRESS, CONTRACT_NAME.to_string())),
        ) / base_token_reserves)
    }
}

fn get_total_value<API: ellipticoin::API>(api: &mut API, token: Token) -> u64 {
    get_base_token_reserves(api, token.clone()) * get_reserves(api, token)
}

fn credit_base_token_reserves<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let base_token_reserves = get_base_token_reserves(api, token.clone());
    set_base_token_reserves(api, token.clone(), base_token_reserves + amount);
}

fn debit_base_token_reserves<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let base_token_reserves = get_base_token_reserves(api, token.clone());
    set_base_token_reserves(api, token.clone(), base_token_reserves - amount);
}

fn pay<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    recipient: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    token::transfer_from(
        api,
        token,
        Address::Contract((SYSTEM_ADDRESS, CONTRACT_NAME.to_string())),
        recipient,
        amount,
    )?;
    Ok(())
}

fn charge<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    address: Address,
    amount: u64,
) -> Result<(), Box<Error>> {
    token::transfer_from(
        api,
        token,
        address,
        Address::Contract((SYSTEM_ADDRESS, CONTRACT_NAME.to_string())),
        amount,
    )?;
    Ok(())
}

fn pool_token(token: Token) -> Token {
    Token {
        issuer: Address::Contract((SYSTEM_ADDRESS, CONTRACT_NAME.to_string())),
        token_id: sha256(token.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{native, *};
    use crate::system_contracts::{
        test_api::{TestAPI, TestState},
        token,
    };
    use ellipticoin::constants::SYSTEM_ADDRESS;
    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB};
    use std::env;
    lazy_static! {
        static ref APPLES: Token = Token {
            issuer: Address::PublicKey(*ALICE),
            token_id: [0; 32]
        };
        static ref BANANAS: Token = Token {
            issuer: Address::PublicKey(*ALICE),
            token_id: [1; 32]
        };
    }

    #[test]
    fn test_add_liqidity() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        token::set_balance(
            &mut api,
            APPLES.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            2 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            2 * BASE_FACTOR,
        );
        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        native::add_liqidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).unwrap();

        assert_eq!(
            token::get_balance(
                &mut api,
                pool_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            2 * BASE_FACTOR
        );
    }

    #[test]
    fn test_swap() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        token::set_balance(
            &mut api,
            APPLES.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BANANAS.clone().into(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            200 * BASE_FACTOR,
        );
        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        native::create_pool(&mut api, BANANAS.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());
        token::set_balance(
            &mut api,
            BANANAS.clone().into(),
            ellipticoin::Address::PublicKey(*BOB),
            100 * BASE_FACTOR,
        );
        native::swap(&mut api, BANANAS.clone(), APPLES.clone(), 100 * BASE_FACTOR).unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            33_249_931
        );
    }

    #[test]
    fn test_swap_base_token() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        token::set_balance(
            &mut api,
            APPLES.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone().into(),
            ellipticoin::Address::PublicKey(*BOB),
            100 * BASE_FACTOR,
        );
        native::swap(
            &mut api,
            BASE_TOKEN.clone(),
            APPLES.clone(),
            100 * BASE_FACTOR,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            49_937_422
        );
    }

    #[test]
    fn test_swap_for_base_token() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        token::set_balance(
            &mut api,
            APPLES.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());
        token::set_balance(
            &mut api,
            APPLES.clone().into(),
            ellipticoin::Address::PublicKey(*BOB),
            100 * BASE_FACTOR,
        );
        native::swap(
            &mut api,
            APPLES.clone(),
            BASE_TOKEN.clone(),
            100 * BASE_FACTOR,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                BASE_TOKEN.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            49_937_422
        );
    }

    #[test]
    fn test_remove_liqidity() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        token::set_balance(
            &mut api,
            APPLES.clone(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BANANAS.clone().into(),
            ellipticoin::Address::PublicKey(*ALICE),
            100 * BASE_FACTOR,
        );
        token::set_balance(
            &mut api,
            BASE_TOKEN.clone().into(),
            ellipticoin::Address::PublicKey(*ALICE),
            200 * BASE_FACTOR,
        );
        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        native::create_pool(&mut api, BANANAS.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());
        token::set_balance(
            &mut api,
            BANANAS.clone(),
            ellipticoin::Address::PublicKey(*BOB),
            100 * BASE_FACTOR,
        );
        native::swap(&mut api, BANANAS.clone(), APPLES.clone(), 100 * BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(ALICE.clone());
        native::remove_liqidity(&mut api, APPLES.clone(), 100 * BASE_FACTOR).unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone().into(),
                ellipticoin::Address::PublicKey(*ALICE)
            ),
            66750069
        );
        assert_eq!(
            token::get_balance(
                &mut api,
                BASE_TOKEN.clone(),
                ellipticoin::Address::PublicKey(*ALICE)
            ),
            199750001
        );
    }
}
