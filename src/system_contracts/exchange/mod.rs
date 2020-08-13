mod constants;
mod errors;

use crate::system_contracts::token::BASE_FACTOR;
use constants::{BASE_TOKEN, FEE};
use ellipticoin::{
    contract_functions::{charge, pay},
    memory_accessors, Address, Token,
};
use std::boxed::Box;
use wasm_rpc::error::Error;
use wasm_rpc_macros::export_native;

memory_accessors!(balance: u64, reserves: u64, base_token_reserves: u64);

export_native! {
    pub fn create_pool<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
        price: u64,
    ) -> Result<(), Box<Error>> {
        charge(api, token.clone(), api.caller(), amount)?;
        credit_reserves(api, token.clone(), amount);
        credit_balance(api, token.clone(), api.caller(), amount);
        charge(api, BASE_TOKEN.clone(), api.caller(), amount * (price / BASE_FACTOR))?;
        Ok(credit_base_token_reserves(api, token, amount))
    }

    pub fn add_liqidity<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        let ratio = get_ratio(api, token.clone())?;
        charge(api, token.clone(), api.caller(), amount)?;
        credit_reserves(api, token.clone(), amount);
        credit_balance(api, token.clone(), api.caller(), amount);
        charge(api, BASE_TOKEN.clone(), api.caller(), amount * (ratio/BASE_FACTOR))?;
        Ok(credit_base_token_reserves(api, token, amount))
    }

    pub fn swap<API: ellipticoin::API>(
        api: &mut API,
        input_token: Token,
        output_token: Token,
        input_amount: u64,
    ) -> Result<(), Box<Error>> {
        let base_token_amount = rebalance_base_token(api, input_token.clone(), apply_fee(input_amount))?;
        charge(api, input_token.clone(), api.caller(), input_amount)?;
        credit_reserves(api, input_token.clone(), input_amount);
        credit_balance(api, input_token.clone(), api.caller(), input_amount);
        debit_base_token_reserves(api, input_token, base_token_amount);
        let token_amount = rebalance(api, output_token.clone(), apply_fee(base_token_amount))?;
        debit_reserves(api, output_token.clone(), token_amount);
        credit_base_token_reserves(api, output_token.clone(), base_token_amount);
        pay(api, output_token, api.caller(), token_amount)
    }
}

fn rebalance<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<u64, Box<Error>> {
    let total = get_total(api, token.clone());
    credit_base_token_reserves(api, token.clone(), amount);
    let new_token_reserves = total / get_base_token_reserves(api, token.clone());
    Ok(get_reserves(api, token.clone()) - new_token_reserves)
}

fn rebalance_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<u64, Box<Error>> {
    let total = get_total(api, token.clone());
    credit_reserves(api, token.clone(), amount);
    let new_base_token_reserves = total / get_reserves(api, token.clone());
    Ok(get_base_token_reserves(api, token.clone()) - new_base_token_reserves)
}

fn apply_fee(amount: u64) -> u64 {
    amount - ((amount * FEE) / BASE_FACTOR)
}

fn get_ratio<API: ellipticoin::API>(api: &mut API, token: Token) -> Result<u64, Box<Error>> {
    let base_token_reserves = get_base_token_reserves(api, token.clone());
    if base_token_reserves == 0 {
        Err(Box::new(errors::POOL_NOT_FOUND.clone()))
    } else {
        Ok(get_reserves(api, token) / base_token_reserves)
    }
}

fn get_total<API: ellipticoin::API>(api: &mut API, token: Token) -> u64 {
    get_base_token_reserves(api, token.clone()) * get_reserves(api, token)
}

fn credit_reserves<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let reserves = get_reserves(api, token.clone());
    set_reserves(api, token.clone(), reserves + amount);
}

fn debit_reserves<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let reserves = get_reserves(api, token.clone());
    set_reserves(api, token.clone(), reserves - amount);
}

fn credit_base_token_reserves<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let base_token_reserves = get_base_token_reserves(api, token.clone());
    set_base_token_reserves(api, token.clone(), base_token_reserves + amount);
}

fn debit_base_token_reserves<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let base_token_reserves = get_base_token_reserves(api, token.clone());
    set_base_token_reserves(api, token.clone(), base_token_reserves - amount);
}

fn credit_balance<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    mut address: Address,
    amount: u64,
) {
    let balance = get_balance(
        api,
        [token.clone().into(), address.clone().to_vec()].concat(),
    );
    set_balance(
        api,
        [token.into(), address.to_vec()].concat(),
        balance + amount,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_contracts::test_api::{TestAPI, TestState};
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
        api.set_balance(APPLES.clone(), *ALICE, 2 * BASE_FACTOR);
        api.set_balance(BASE_TOKEN.clone(), *ALICE, 2 * BASE_FACTOR);
        create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        add_liqidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).unwrap();

        assert_eq!(
            get_balance(
                &mut api,
                [APPLES.clone().into(), Address::PublicKey(*ALICE).to_vec()].concat()
            ),
            2 * BASE_FACTOR
        );
    }

    #[test]
    fn test_swap() {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));
        let mut state = TestState::new();
        let mut api = TestAPI::new(&mut state, *ALICE, (SYSTEM_ADDRESS, "Token".to_string()));
        api.set_balance(APPLES.clone(), *ALICE, 100 * BASE_FACTOR);
        api.set_balance(BANANAS.clone(), *ALICE, 100 * BASE_FACTOR);
        api.set_balance(BASE_TOKEN.clone(), *ALICE, 200 * BASE_FACTOR);
        create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        create_pool(&mut api, BANANAS.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());
        api.set_balance(BANANAS.clone(), *BOB, 100 * BASE_FACTOR);
        swap(&mut api, BANANAS.clone(), APPLES.clone(), 100 * BASE_FACTOR).unwrap();
        assert_eq!(api.get_balance(APPLES.clone(), *BOB), 33_249_931);
    }
}
