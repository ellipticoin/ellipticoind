pub mod constants;
mod errors;

use crate::{
    helpers::sha256,
    system_contracts::token::{self, BASE_FACTOR},
};
use constants::{BASE_TOKEN, FEE};
use ellipticoin::{charge, pay, state_accessors, Address, Token};
use std::{boxed::Box, collections::HashSet, str};
use wasm_rpc::error::Error;
use wasm_rpc_macros::export_native;

pub const CONTRACT_NAME: &'static str = "Exchange";
lazy_static! {
    pub static ref ADDRESS: std::string::String = CONTRACT_NAME.to_string();
}

state_accessors!(
    pool_supply_of_base_token(token: Token) -> u64;
    pool_supply_of_token(token: Token) -> u64;
    share_holders(token: Token) -> HashSet<Address>;
);

fn mint<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) -> Result<(), Box<Error>> {
    token::mint(api, liquidity_token(token.clone()), api.caller(), amount)?;
    let mut share_holders = get_share_holders(api, token.clone());
    share_holders.insert(api.caller());
    set_share_holders(api, token, share_holders);
    Ok(())
}

export_native! {
    pub fn create_pool<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
        starting_price: u64,
    ) -> Result<(), Box<Error>> {
        validate_pool_does_not_exist(api, token.clone())?;
        charge!(api, token.clone(), api.caller(), amount)?;
        credit_pool_supply_of_token(api, token.clone(), amount);
        charge!(api, BASE_TOKEN.clone(), api.caller(), ((amount as u128 * starting_price as u128) / BASE_FACTOR as u128) as u64)?;
        credit_pool_supply_of_base_token(api, token.clone(), (amount * starting_price) / BASE_FACTOR);
        mint(api, token, amount)?;
        Ok(())
    }

    pub fn add_liquidity<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        validate_pool_exists(api, token.clone())?;
        let pool_supply_of_token = get_pool_supply_of_token(api, token.clone());
        let pool_supply_of_base_token = get_pool_supply_of_base_token(api, token.clone());
        let total_supply_of_liquidity_token = token::get_total_supply(api, liquidity_token(token.clone()));

        let mint_amount = (amount as u128 * total_supply_of_liquidity_token as u128 / pool_supply_of_token as u128) as u64;

        charge!(api, token.clone(), api.caller(), amount)?;
        credit_pool_supply_of_token(api, token.clone(), amount);

        charge!(api, BASE_TOKEN.clone(), api.caller(), ((amount as u128 * pool_supply_of_base_token as u128)/pool_supply_of_token as u128) as u64)?;
        credit_pool_supply_of_base_token(api, token.clone(), (amount * pool_supply_of_base_token)/pool_supply_of_token);

        mint(api, token, mint_amount)?;
        Ok(())
    }

    pub fn remove_liquidity<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        percentage: u64,
    ) -> Result<(), Box<Error>> {
        let pool_supply_of_token = get_pool_supply_of_token(api, token.clone());
        let pool_supply_of_base_token = get_pool_supply_of_base_token(api, token.clone());
        let liquidity_token_balance = token::get_balance(api, liquidity_token(token.clone()), api.caller());
        let total_supply_of_liquidity_token = token::get_total_supply(api, liquidity_token(token.clone()));
        let token_balance = pool_supply_of_token as u128 * liquidity_token_balance as u128 / total_supply_of_liquidity_token as u128;
        let base_token_balance = pool_supply_of_base_token as u128 * liquidity_token_balance as u128 / total_supply_of_liquidity_token as u128;

        burn_liquidity(api, token.clone(), (liquidity_token_balance as u128 * percentage as u128 / BASE_FACTOR as u128) as u64)?;
        debit_pool_supply_of_base_token(api, token.clone(), (base_token_balance as u128 * percentage as u128 / BASE_FACTOR as u128) as u64)?;
        pay!(api, BASE_TOKEN.clone(), api.caller(), (base_token_balance as u128 * percentage as u128 / BASE_FACTOR as u128) as u64)?;
        debit_pool_supply_of_token(api, token.clone(), (percentage as u128 * token_balance as u128/ BASE_FACTOR as u128) as u64)?;
        pay!(api, token, api.caller(), (percentage as u128 * token_balance as u128 / BASE_FACTOR as u128) as u64)?;

        Ok(())
    }

    pub fn exchange<API: ellipticoin::API>(
        api: &mut API,
        input_token: Token,
        output_token: Token,
        input_amount: u64,
        minimum_output_token_amount: u64
    ) -> Result<(), Box<Error>> {
        charge!(api, input_token.clone(), api.caller(), input_amount)?;
        let base_token_amount = exchange_token_for_base_token(api, input_token, input_amount)?;
        let output_token_amount = exchange_base_token_for_token(api, output_token.clone(), base_token_amount)?;
        if output_token_amount < minimum_output_token_amount {
            return Err(Box::new(errors::MAX_SLIPPAGE_EXCEEDED.clone()))
        }
        pay!(api, output_token, api.caller(), output_token_amount)?;
        Ok(())
    }
}

fn exchange_token_for_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<u64, Box<Error>> {
    if token == BASE_TOKEN.clone() {
        return Ok(amount);
    };
    validate_pool_exists(api, token.clone())?;
    let amount_minus_fee = apply_fee(amount);
    let base_token_output_amount = calculate_output_amount(
        get_pool_supply_of_token(api, token.clone()),
        get_pool_supply_of_base_token(api, token.clone()),
        amount_minus_fee,
    );
    credit_pool_supply_of_token(api, token.clone(), amount);
    debit_pool_supply_of_base_token(api, token.clone(), base_token_output_amount)?;
    Ok(base_token_output_amount)
}

fn exchange_base_token_for_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<u64, Box<Error>> {
    if token == BASE_TOKEN.clone() {
        return Ok(amount);
    };
    validate_pool_exists(api, token.clone())?;
    let amount_minus_fee = apply_fee(amount);
    let output_amount = calculate_output_amount(
        get_pool_supply_of_base_token(api, token.clone()),
        get_pool_supply_of_token(api, token.clone()),
        amount_minus_fee,
    );
    debit_pool_supply_of_token(api, token.clone(), output_amount)?;
    credit_pool_supply_of_base_token(api, token.clone(), amount);
    Ok(output_amount)
}

fn calculate_output_amount(input_supply: u64, output_supply: u64, input_amount: u64) -> u64 {
    let invariant = input_supply as u128 * output_supply as u128;
    let new_output_supply =
        (invariant / (input_supply as u128 + input_amount as u128) as u128) as u64;
    output_supply - new_output_supply
}

fn apply_fee(amount: u64) -> u64 {
    amount - ((amount as u128 * FEE as u128) / BASE_FACTOR as u128) as u64
}

fn validate_pool_does_not_exist<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
) -> Result<(), Box<Error>> {
    if get_pool_supply_of_base_token(api, token.clone()) != 0 {
        Err(Box::new(errors::POOL_ALREADY_EXISTS.clone()))
    } else {
        Ok(())
    }
}

fn validate_pool_exists<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
) -> Result<(), Box<Error>> {
    if get_pool_supply_of_token(api, token.clone()) > 0 {
        Ok(())
    } else {
        Err(Box::new(errors::POOL_NOT_FOUND.clone()))
    }
}

pub fn get_price<API: ellipticoin::API>(api: &mut API, token: Token) -> Result<u64, Box<Error>> {
    let pool_supply_of_base_token = get_pool_supply_of_base_token(api, token.clone());
    let pool_supply_of_token = get_pool_supply_of_token(api, token.clone());
    if pool_supply_of_token == 0 {
        Err(Box::new(errors::POOL_NOT_FOUND.clone()))
    } else {
        Ok(pool_supply_of_base_token * BASE_FACTOR / pool_supply_of_token)
    }
}

fn credit_pool_supply_of_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) {
    let base_token_supply = get_pool_supply_of_base_token(api, token.clone());
    set_pool_supply_of_base_token(api, token.clone(), base_token_supply + amount);
}

pub fn debit_pool_supply_of_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<(), Box<Error>> {
    let base_token_supply = get_pool_supply_of_base_token(api, token.clone());
    if amount <= base_token_supply {
        Ok(set_pool_supply_of_base_token(
            api,
            token.clone(),
            base_token_supply - amount,
        ))
    } else {
        Err(Box::new(token::errors::INSUFFICIENT_FUNDS.clone()))
    }
}

fn credit_pool_supply_of_token<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let token_supply = get_pool_supply_of_token(api, token.clone());
    set_pool_supply_of_token(api, token.clone(), token_supply + amount);
}

pub fn debit_pool_supply_of_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<(), Box<Error>> {
    let token_supply = get_pool_supply_of_token(api, token.clone());
    if amount <= token_supply {
        Ok(set_pool_supply_of_token(
            api,
            token.clone(),
            token_supply - amount,
        ))
    } else {
        Err(Box::new(token::errors::INSUFFICIENT_FUNDS.clone()))
    }
}

pub fn burn_liquidity<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<(), Box<Error>> {
    token::burn(api, liquidity_token(token.clone()), api.caller(), amount)?;
    if token::get_balance(api, liquidity_token(token.clone()), api.caller()) == 0 {
        let mut share_holders = get_share_holders(api, token.clone());
        share_holders.remove(&api.caller());
        set_share_holders(api, token.clone(), share_holders);
    }
    Ok(())
}

pub fn liquidity_token(token: Token) -> Token {
    Token {
        issuer: Address::Contract(CONTRACT_NAME.to_string()),
        id: sha256(token.into()).to_vec().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{native, *};
    use crate::system_contracts::token::{self, constants::ELC};
    use ellipticoin::API;
    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB},
            tokens::{APPLES, BANANAS},
        },
        setup,
    };
    use std::collections::HashMap;

    #[test]
    fn test_create_pool() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                        ellipticoin::Address::PublicKey(*ALICE) =>
                            vec![
                                (APPLES.clone(), 1 * BASE_FACTOR),
                            (BASE_TOKEN.clone(), 1 * BASE_FACTOR),

            ]
                    },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();

        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            1 * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![Address::PublicKey(*ALICE)]
        );
    }

    #[test]
    fn test_recreate_pool() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 2 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 2 * BASE_FACTOR),
                ],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        match native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR) {
            Ok(_) => assert!(false, "Should have received error recreating pool!"),
            Err(x) => assert!(
                (*x).code == errors::POOL_ALREADY_EXISTS.code,
                "Should have received pool already exists error!"
            ),
        }

        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            1 * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![Address::PublicKey(*ALICE)]
        );
    }

    #[test]
    fn test_create_pool_insufficient_base_token_funds() {
        let apple_balance = 2;
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), apple_balance * BASE_FACTOR),
                    (BASE_TOKEN.clone(), apple_balance * BASE_FACTOR / 2),
                ]
            },
            &mut state,
        );

        assert!(native::create_pool(
            &mut api,
            APPLES.clone(),
            apple_balance * BASE_FACTOR,
            apple_balance * BASE_FACTOR
        )
        .is_err());
        api.revert();

        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            0
        );
        assert_eq!(
            token::get_balance(&mut api, APPLES.clone(), Address::PublicKey(*ALICE)),
            apple_balance * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![]
        );
    }

    #[test]
    fn test_create_pool_insufficient_token_funds() {
        let apple_balance = 1;
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), apple_balance * BASE_FACTOR),
                    (BASE_TOKEN.clone(), apple_balance * BASE_FACTOR * 2),
                ],
            },
            &mut state,
        );

        assert!(native::create_pool(
            &mut api,
            APPLES.clone(),
            apple_balance * 2 * BASE_FACTOR,
            apple_balance * 2 * BASE_FACTOR
        )
        .is_err());

        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            0
        );
        assert_eq!(
            token::get_balance(&mut api, APPLES.clone(), Address::PublicKey(*ALICE)),
            apple_balance * BASE_FACTOR
        );
        assert_eq!(
            token::get_balance(&mut api, BASE_TOKEN.clone(), Address::PublicKey(*ALICE)),
            apple_balance * 2 * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![]
        );
    }

    #[test]
    fn test_add_liquidity() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 2 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 2 * BASE_FACTOR),
                ],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        native::add_liquidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).unwrap();

        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            2 * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![Address::PublicKey(*ALICE)]
        );
    }

    #[test]
    fn test_add_to_existing_liquidity() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 3 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 3 * BASE_FACTOR),
                ]
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        native::add_liquidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).unwrap();
        native::add_liquidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).unwrap();

        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            3 * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![Address::PublicKey(*ALICE)]
        );
    }

    #[test]
    fn test_add_liquidity_insufficient_base_token_funds() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 2 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 1 * BASE_FACTOR),
                ],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        api.commit();
        assert!(native::add_liquidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).is_err());
        api.revert();

        assert_eq!(
            token::get_balance(&mut api, APPLES.clone(), Address::PublicKey(*ALICE)),
            1 * BASE_FACTOR
        );
        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            1 * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![Address::PublicKey(*ALICE)]
        );
    }

    #[test]
    fn test_add_liquidity_insufficient_token_funds() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 1 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 2 * BASE_FACTOR),
                ]
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        assert!(native::add_liquidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).is_err());

        assert_eq!(
            token::get_balance(&mut api, APPLES.clone(), Address::PublicKey(*ALICE)),
            0
        );
        assert_eq!(
            token::get_balance(&mut api, BASE_TOKEN.clone(), Address::PublicKey(*ALICE)),
            1 * BASE_FACTOR
        );
        assert_eq!(
            token::get_balance(
                &mut api,
                liquidity_token(APPLES.clone()),
                Address::PublicKey(*ALICE)
            ),
            1 * BASE_FACTOR
        );
        assert_eq!(
            get_share_holders(&mut api, APPLES.clone(),)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![Address::PublicKey(*ALICE)]
        );
    }

    #[test]
    fn test_remove_liquidity() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 100 * BASE_FACTOR),
                    (BANANAS.clone(), 100 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 200 * BASE_FACTOR),
                ],
                ellipticoin::Address::PublicKey(*BOB) =>
                vec![(BANANAS.clone(), 100 * BASE_FACTOR)],
                ellipticoin::Address::Contract(ADDRESS.clone()) =>
                vec![(ELC.clone(), 100 * BASE_FACTOR)],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        let apples_in_pool = get_pool_supply_of_token(&mut api, APPLES.clone().into());
        assert_eq!(apples_in_pool, 100 * BASE_FACTOR);
        let base_token_in_pool = get_pool_supply_of_base_token(&mut api, APPLES.clone().into());
        assert_eq!(base_token_in_pool, 100 * BASE_FACTOR);

        native::create_pool(&mut api, BANANAS.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        let bananas_in_pool = get_pool_supply_of_token(&mut api, BANANAS.clone().into());
        assert_eq!(bananas_in_pool, 100 * BASE_FACTOR);
        let base_token_in_pool = get_pool_supply_of_base_token(&mut api, BANANAS.clone().into());
        assert_eq!(base_token_in_pool, 100 * BASE_FACTOR);

        native::remove_liquidity(&mut api, APPLES.clone(), BASE_FACTOR).unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone().into(),
                ellipticoin::Address::PublicKey(*ALICE)
            ),
            100 * BASE_FACTOR
        );
        assert_eq!(
            token::get_balance(
                &mut api,
                BASE_TOKEN.clone(),
                ellipticoin::Address::PublicKey(*ALICE)
            ),
            100 * BASE_FACTOR
        );

        let bananas_in_pool = get_pool_supply_of_token(&mut api, BANANAS.clone().into());
        assert_eq!(bananas_in_pool, 100 * BASE_FACTOR);
        let base_token_in_pool = get_pool_supply_of_base_token(&mut api, BANANAS.clone().into());
        assert_eq!(base_token_in_pool, 100 * BASE_FACTOR);
    }

    #[test]
    fn test_remove_liquidity_after_trade() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                    ellipticoin::Address::PublicKey(*ALICE) =>
                    vec![
                        (APPLES.clone(), 100 * BASE_FACTOR),
                        (BANANAS.clone(), 100 * BASE_FACTOR),
                        (BASE_TOKEN.clone(), 200 * BASE_FACTOR),
                    ],
                    ellipticoin::Address::PublicKey(*BOB) =>
                    vec![(BANANAS.clone(), 100 * BASE_FACTOR)],
                    ellipticoin::Address::Contract(ADDRESS.clone()) =>
                    vec![(ELC.clone(), 100 * BASE_FACTOR)],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        native::create_pool(&mut api, BANANAS.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());
        native::exchange(
            &mut api,
            BANANAS.clone(),
            APPLES.clone(),
            100 * BASE_FACTOR,
            0,
        )
        .unwrap();

        api.caller = Address::PublicKey(ALICE.clone());
        let apples_in_pool = get_pool_supply_of_token(&mut api, APPLES.clone().into());
        let base_token_in_pool = get_pool_supply_of_base_token(&mut api, APPLES.clone().into());

        native::remove_liquidity(&mut api, APPLES.clone(), BASE_FACTOR).unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone().into(),
                ellipticoin::Address::PublicKey(*ALICE)
            ),
            apples_in_pool
        );
        assert_eq!(
            token::get_balance(
                &mut api,
                BASE_TOKEN.clone(),
                ellipticoin::Address::PublicKey(*ALICE)
            ),
            base_token_in_pool
        );
    }

    #[test]
    fn test_remove_liquidity_insufficient_liquidity() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 1 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 1 * BASE_FACTOR),
                ],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, BASE_FACTOR).unwrap();
        assert!(native::remove_liquidity(&mut api, APPLES.clone(), 2 * BASE_FACTOR).is_err());
    }

    #[test]
    fn test_exchange() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                    ellipticoin::Address::PublicKey(*ALICE) =>
                    vec![
                        (APPLES.clone(), 100 * BASE_FACTOR),
                        (BANANAS.clone(), 100 * BASE_FACTOR),
                        (BASE_TOKEN.clone(), 200 * BASE_FACTOR),
                    ],
                    ellipticoin::Address::PublicKey(*BOB) =>
                    vec![(APPLES.clone(), 100 * BASE_FACTOR)],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        native::create_pool(&mut api, BANANAS.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();

        api.caller = Address::PublicKey(BOB.clone());
        native::exchange(
            &mut api,
            APPLES.clone(),
            BANANAS.clone(),
            100 * BASE_FACTOR,
            0,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                BANANAS.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            33_233_234
        );
    }

    #[test]
    fn test_exchange_invariant_overflow() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                    ellipticoin::Address::PublicKey(*ALICE) =>
                    vec![
                        (APPLES.clone(), 100_000 * BASE_FACTOR),
                        (BASE_TOKEN.clone(), 1_000 * BASE_FACTOR),
                    ],
                    ellipticoin::Address::PublicKey(*BOB) =>
                    vec![(APPLES.clone(), 100_000 * BASE_FACTOR)],
            },
            &mut state,
        );

        native::create_pool(
            &mut api,
            APPLES.clone(),
            100_000 * BASE_FACTOR,
            BASE_FACTOR / 100,
        )
        .unwrap();

        api.caller = Address::PublicKey(BOB.clone());
        native::exchange(
            &mut api,
            APPLES.clone(),
            BASE_TOKEN.clone(),
            100 * BASE_FACTOR,
            0,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                BASE_TOKEN.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            996_007
        );
        native::exchange(&mut api, BASE_TOKEN.clone(), APPLES.clone(), 996_007, 0).unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            99_999_401_499
        );
        assert_eq!(
            token::get_balance(
                &mut api,
                BASE_TOKEN.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            0
        );
    }

    #[test]
    fn test_exchange_base_token() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                     ellipticoin::Address::PublicKey(*ALICE) =>
                     vec![
                         (APPLES.clone(), 100 * BASE_FACTOR),
                         (BASE_TOKEN.clone(), 100 * BASE_FACTOR),
                     ],
                     ellipticoin::Address::PublicKey(*BOB) =>
                     vec![(BASE_TOKEN.clone(), 100 * BASE_FACTOR)],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());
        credit_pool_supply_of_base_token(&mut api, BASE_TOKEN.clone(), 100 * BASE_FACTOR);

        native::exchange(
            &mut api,
            BASE_TOKEN.clone(),
            APPLES.clone(),
            100 * BASE_FACTOR,
            0,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            49_924_888
        );
    }

    #[test]
    fn test_exchange_for_base_token() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                    ellipticoin::Address::PublicKey(*ALICE) =>
                    vec![
                        (APPLES.clone(), 100 * BASE_FACTOR),
                        (BASE_TOKEN.clone(), 100 * BASE_FACTOR),
                    ],
                    ellipticoin::Address::PublicKey(*BOB) =>
                    vec![(APPLES.clone(), 100 * BASE_FACTOR)],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();

        api.caller = Address::PublicKey(BOB.clone());
        native::exchange(
            &mut api,
            APPLES.clone(),
            BASE_TOKEN.clone(),
            100 * BASE_FACTOR,
            0,
        )
        .unwrap();
        assert_eq!(
            token::get_balance(
                &mut api,
                BASE_TOKEN.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            49_924_888
        );
    }

    #[test]
    fn test_exchange_max_slippage_exceeded() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                    ellipticoin::Address::PublicKey(*ALICE) =>
                    vec![
                        (APPLES.clone(), 100 * BASE_FACTOR),
                        (BANANAS.clone(), 100 * BASE_FACTOR),
                        (BASE_TOKEN.clone(), 200 * BASE_FACTOR),
                    ],
                    ellipticoin::Address::PublicKey(*BOB) =>
                    vec![(APPLES.clone(), 100 * BASE_FACTOR)],
            },
            &mut state,
        );

        native::create_pool(&mut api, APPLES.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        native::create_pool(&mut api, BANANAS.clone(), 100 * BASE_FACTOR, BASE_FACTOR).unwrap();
        api.caller = Address::PublicKey(BOB.clone());

        match native::exchange(
            &mut api,
            APPLES.clone(),
            BANANAS.clone(),
            100 * BASE_FACTOR,
            33_233_235,
        ) {
            Err(x) => assert!(
                x.code == errors::MAX_SLIPPAGE_EXCEEDED.code,
                "Should have resulted in a max slippage error!"
            ),
            _ => assert!(false, "Should have resulted in a max slippage error!"),
        };
    }

    #[test]
    fn test_exchange_no_pool_exists() {
        let mut state = HashMap::new();
        let mut api = setup(
            hashmap! {
                ellipticoin::Address::PublicKey(*ALICE) =>
                vec![
                    (APPLES.clone(), 100 * BASE_FACTOR),
                    (BANANAS.clone(), 100 * BASE_FACTOR),
                    (BASE_TOKEN.clone(), 200 * BASE_FACTOR),
                ],
                ellipticoin::Address::PublicKey(*BOB) =>
                vec![(APPLES.clone(), 100 * BASE_FACTOR)],
            },
            &mut state,
        );

        api.caller = Address::PublicKey(BOB.clone());
        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            100 * BASE_FACTOR
        );
        let exchange_res = native::exchange(
            &mut api,
            APPLES.clone(),
            BANANAS.clone(),
            100 * BASE_FACTOR,
            0,
        );
        if exchange_res.is_err() {
            api.revert();
        }
        match exchange_res {
            Err(x) => assert!(
                (*x).code == errors::POOL_NOT_FOUND.code,
                "Should have returned pool not found error"
            ),
            _ => assert!(
                false,
                "Should not have been able to exchange without a pool!"
            ),
        };

        assert_eq!(
            token::get_balance(
                &mut api,
                APPLES.clone(),
                ellipticoin::Address::PublicKey(*BOB)
            ),
            100 * BASE_FACTOR
        );
    }
}
