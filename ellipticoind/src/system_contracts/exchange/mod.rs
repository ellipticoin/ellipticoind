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

const CONTRACT_NAME: &'static str = "Exchange";
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
        validate_token_amount(api, amount, token.clone())?;
        validate_base_token_amount(api, (amount * starting_price)/BASE_FACTOR)?;
        validate_pool_does_not_exist(api, token.clone())?;

        charge!(api, token.clone(), api.caller(), amount)?;
        credit_pool_supply_of_token(api, token.clone(), amount);
        charge!(api, BASE_TOKEN.clone(), api.caller(), (amount * starting_price) / BASE_FACTOR)?;
        credit_pool_supply_of_base_token(api, token.clone(), (amount * starting_price) / BASE_FACTOR);
        mint(api, token, amount)?;
        Ok(())
    }

    pub fn add_liquidity<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        validate_token_amount(api, amount, token.clone())?;

        let price = get_price(api, token.clone())?;
        validate_base_token_amount(api, (amount * price)/BASE_FACTOR)?;

        let current_token_balance = get_pool_supply_of_token(api, token.clone());
        let current_liquidity_token_balance = token::get_total_supply(api, liquidity_token(token.clone()));

        let amount_to_mint: u64 = amount * current_liquidity_token_balance / current_token_balance;

        charge!(api, token.clone(), api.caller(), amount)?;
        credit_pool_supply_of_token(api, token.clone(), amount);

        charge!(api, BASE_TOKEN.clone(), api.caller(), (amount * price)/BASE_FACTOR)?;
        credit_pool_supply_of_base_token(api, token.clone(), (amount * price)/BASE_FACTOR);

        mint(api, token, amount_to_mint)?;
        Ok(())
    }

    pub fn remove_liquidity<API: ellipticoin::API>(
        api: &mut API,
        token: Token,
        amount: u64,
    ) -> Result<(), Box<Error>> {
        validate_liquidity_amount(api, token.clone(), amount)?;
        let token_balance = get_pool_supply_of_token(api, token.clone());
        let base_token_balance = get_pool_supply_of_base_token(api, token.clone());
        let liquidity_token_supply = token::get_total_supply(api, liquidity_token(token.clone()));

        debit_pool_supply_of_base_token(api, token.clone(), base_token_balance * amount / token_balance);
        pay!(api, BASE_TOKEN.clone(), api.caller(), base_token_balance * amount / token_balance)?;
        debit_pool_supply_of_token(api, token.clone(), amount);
        pay!(api, token.clone(), api.caller(), amount)?;

        burn(api, token, liquidity_token_supply * amount / token_balance)?;

        Ok(())
    }

    pub fn exchange<API: ellipticoin::API>(
        api: &mut API,
        input_token: Token,
        output_token: Token,
        input_amount: u64,
        minimum_output_token_amount: u64
    ) -> Result<(), Box<Error>> {
        let mut book_input_entry = false;
        let mut book_output_entry = false;

        let input_amount_in_base_token = if input_token == BASE_TOKEN.clone() {
            input_amount
        } else {
            validate_pool_exists(api, input_token.clone())?;
            book_input_entry = true;
            calculate_input_amount_in_base_token(api, input_token.clone(), apply_fee(input_amount))?
        };

        let output_token_amount = if output_token == BASE_TOKEN.clone() {
            input_amount_in_base_token
        } else {
            validate_pool_exists(api, output_token.clone())?;
            book_output_entry = true;
            calculate_amount_in_output_token(api, output_token.clone(), apply_fee(input_amount_in_base_token))?
        };
        if output_token_amount < minimum_output_token_amount {
            return Err(Box::new(errors::MAX_SLIPPAGE_EXCEEDED.clone()))
        }
        charge!(api, input_token.clone(), api.caller(), input_amount)?;
        if book_input_entry {
            credit_pool_supply_of_token(api, input_token.clone(), input_amount);
            debit_pool_supply_of_base_token(api, input_token.clone(), input_amount_in_base_token);
        }
        if book_output_entry {
            credit_pool_supply_of_base_token(api, output_token.clone(), input_amount_in_base_token);
            debit_pool_supply_of_token(api, output_token.clone(), output_token_amount);
        }
        pay!(api, output_token.clone(), api.caller(), output_token_amount)?;

        Ok(())
    }
}

fn burn<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) -> Result<(), Box<Error>> {
    token::burn(api, liquidity_token(token.clone()), api.caller(), amount)?;
    if token::get_balance(api, liquidity_token(token.clone()), api.caller()) == 0 {
        let mut share_holders = get_share_holders(api, token.clone());
        share_holders.remove(&api.caller());
        set_share_holders(api, token.clone(), share_holders);
    }
    Ok(())
}

fn calculate_input_amount_in_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<u64, Box<Error>> {
    let invariant = get_pool_invariant(api, token.clone());
    let divisor = get_pool_supply_of_token(api, token.clone()) + amount;
    if divisor == 0 {
        return Ok(0);
    }
    let new_base_token_supply = invariant / divisor;
    Ok(get_pool_supply_of_base_token(api, token.clone()) - new_base_token_supply)
}

fn calculate_amount_in_output_token<API: ellipticoin::API>(
    api: &mut API,
    output_token: Token,
    amount_in_base_token: u64,
) -> Result<u64, Box<Error>> {
    let invariant = get_pool_invariant(api, output_token.clone());
    let divisor = get_pool_supply_of_base_token(api, output_token.clone()) + amount_in_base_token;
    if divisor == 0 {
        return Ok(0);
    }
    let new_token_supply = invariant / divisor;
    Ok(get_pool_supply_of_token(api, output_token.clone()) - new_token_supply)
}

fn apply_fee(amount: u64) -> u64 {
    amount - ((amount * FEE) / BASE_FACTOR)
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
    if get_pool_supply_of_base_token(api, token.clone()) == 0 {
        Err(Box::new(errors::POOL_NOT_FOUND.clone()))
    } else {
        Ok(())
    }
}

fn validate_base_token_amount<API: ellipticoin::API>(
    api: &mut API,
    amount: u64,
) -> Result<(), Box<Error>> {
    let base_token_balance = token::get_balance(api, BASE_TOKEN.clone(), api.caller());
    if amount > base_token_balance {
        Err(Box::new(token::errors::INSUFFICIENT_FUNDS.clone()))
    } else {
        Ok(())
    }
}

fn validate_token_amount<API: ellipticoin::API>(
    api: &mut API,
    amount: u64,
    token: Token,
) -> Result<(), Box<Error>> {
    let token_balance = token::get_balance(api, token.clone(), api.caller());
    if amount > token_balance {
        Err(Box::new(token::errors::INSUFFICIENT_FUNDS.clone()))
    } else {
        Ok(())
    }
}

fn validate_liquidity_amount<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<(), Box<Error>> {
    let liquidity_token_supply = token::get_total_supply(api, liquidity_token(token.clone()));
    if liquidity_token_supply == 0 {
        return Err(Box::new(token::errors::INSUFFICIENT_FUNDS.clone()));
    }

    let token_supply = get_pool_supply_of_token(api, token.clone());
    let requester_liquidity_token_balance =
        token::get_balance(api, liquidity_token(token.clone()), api.caller());

    if amount / token_supply > requester_liquidity_token_balance / liquidity_token_supply {
        Err(Box::new(token::errors::INSUFFICIENT_FUNDS.clone()))
    } else {
        Ok(())
    }
}

pub fn get_price<API: ellipticoin::API>(api: &mut API, token: Token) -> Result<u64, Box<Error>> {
    let base_token_supply = get_pool_supply_of_base_token(api, token.clone());
    if base_token_supply == 0 {
        Err(Box::new(errors::POOL_NOT_FOUND.clone()))
    } else {
        Ok(base_token_supply * BASE_FACTOR / get_pool_supply_of_token(api, token))
    }
}

fn get_pool_invariant<API: ellipticoin::API>(api: &mut API, token: Token) -> u64 {
    let base_token_supply = get_pool_supply_of_base_token(api, token.clone());
    let token_supply = get_pool_supply_of_token(api, token);
    base_token_supply * token_supply
}

fn credit_pool_supply_of_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) {
    let base_token_supply = get_pool_supply_of_base_token(api, token.clone());
    set_pool_supply_of_base_token(api, token.clone(), base_token_supply + amount);
}

fn debit_pool_supply_of_base_token<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) {
    let base_token_supply = get_pool_supply_of_base_token(api, token.clone());
    set_pool_supply_of_base_token(api, token.clone(), base_token_supply - amount);
}

fn credit_pool_supply_of_token<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let token_supply = get_pool_supply_of_token(api, token.clone());
    set_pool_supply_of_token(api, token.clone(), token_supply + amount);
}

fn debit_pool_supply_of_token<API: ellipticoin::API>(api: &mut API, token: Token, amount: u64) {
    let token_supply = get_pool_supply_of_token(api, token.clone());
    set_pool_supply_of_token(api, token.clone(), token_supply - amount);
}

pub fn liquidity_token(token: Token) -> Token {
    Token {
        issuer: Address::Contract(CONTRACT_NAME.to_string()),
        id: sha256(token.into()).to_vec().into(),
    }
}

pub fn price<API: ellipticoin::API>(api: &mut API, token: Token) -> u64 {
    if token == BASE_TOKEN.clone() {
        return BASE_FACTOR;
    }

    let base_token_supply = get_pool_supply_of_base_token(api, token.clone());
    let token_supply = get_pool_supply_of_token(api, token.clone());
    if base_token_supply == 0 || token_supply == 0 {
        0
    } else {
        base_token_supply * BASE_FACTOR / token_supply
    }
}

pub fn share_of_pool<API: ellipticoin::API>(api: &mut API, token: Token, address: Address) -> u32 {
    let balance = token::get_balance(api, liquidity_token(token.clone()), address);
    let total_supply = token::get_total_supply(api, liquidity_token(token.clone()));
    if balance == 0 {
        return 0;
    }
    (balance * BASE_FACTOR / total_supply) as u32
}

#[cfg(test)]
mod tests {
    use super::{native, *};
    use crate::system_contracts::{test_api::TestAPI, token};
    use ellipticoin::constants::ELC;
    use ellipticoin_test_framework::constants::actors::{ALICE, ALICES_PRIVATE_KEY, BOB};
    use std::{collections::HashMap, env};

    lazy_static! {
        static ref APPLES: Token = Token {
            issuer: Address::PublicKey(*ALICE),
            id: vec![0].into()
        };
        static ref BANANAS: Token = Token {
            issuer: Address::PublicKey(*ALICE),
            id: vec![1].into()
        };
    }

    struct TokenBalance {
        token: Token,
        balance: u64,
    }
    impl TokenBalance {
        pub fn new(token: Token, balance: u64) -> Self {
            Self { token, balance }
        }
    }

    struct AddressBalance {
        address: ellipticoin::Address,
        balances: Vec<TokenBalance>,
    }
    impl AddressBalance {
        pub fn new(address: ellipticoin::Address, balances: Vec<TokenBalance>) -> Self {
            Self { address, balances }
        }
    }

    fn setup(balances: Vec<AddressBalance>, state: &mut HashMap<Vec<u8>, Vec<u8>>) -> TestAPI {
        env::set_var("PRIVATE_KEY", base64::encode(&ALICES_PRIVATE_KEY[..]));

        let mut api = TestAPI::new(state, *ALICE, "Token".to_string());

        for person_balance in balances.iter() {
            for token_balance in person_balance.balances.iter() {
                token::set_balance(
                    &mut api,
                    token_balance.token.clone(),
                    person_balance.address.clone(),
                    token_balance.balance * BASE_FACTOR,
                );
            }
        }

        api
    }

    #[test]
    fn test_create_pool() {
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), 1),
                TokenBalance::new(BASE_TOKEN.clone(), 1),
            ],
        )];
        let mut state = HashMap::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), 2),
                TokenBalance::new(BASE_TOKEN.clone(), 2),
            ],
        )];
        let mut state = HashMap::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), apple_balance),
                TokenBalance::new(BASE_TOKEN.clone(), apple_balance / 2),
            ],
        )];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

        assert!(native::create_pool(
            &mut api,
            APPLES.clone(),
            apple_balance * BASE_FACTOR,
            apple_balance * BASE_FACTOR
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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), apple_balance),
                TokenBalance::new(BASE_TOKEN.clone(), apple_balance * 2),
            ],
        )];
        let mut state = HashMap::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), 2),
                TokenBalance::new(BASE_TOKEN.clone(), 2),
            ],
        )];
        let mut state = HashMap::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), 3),
                TokenBalance::new(BASE_TOKEN.clone(), 3),
            ],
        )];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), 2),
                TokenBalance::new(BASE_TOKEN.clone(), 1),
            ],
        )];
        let mut state = HashMap::new();
        let mut api = setup(balances, &mut state);

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, 1 * BASE_FACTOR).unwrap();
        assert!(native::add_liquidity(&mut api, APPLES.clone(), 1 * BASE_FACTOR).is_err());

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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), 1),
                TokenBalance::new(BASE_TOKEN.clone(), 2),
            ],
        )];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*ALICE),
                vec![
                    TokenBalance::new(APPLES.clone(), 100),
                    TokenBalance::new(BANANAS.clone(), 100),
                    TokenBalance::new(BASE_TOKEN.clone(), 200),
                ],
            ),
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*BOB),
                vec![TokenBalance::new(BANANAS.clone(), 100)],
            ),
            AddressBalance::new(
                ellipticoin::Address::Contract(ADDRESS.clone()),
                vec![TokenBalance::new(ELC.clone(), 100)],
            ),
        ];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

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

        native::remove_liquidity(&mut api, APPLES.clone(), apples_in_pool).unwrap();
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
        let balances: Vec<AddressBalance> = vec![
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*ALICE),
                vec![
                    TokenBalance::new(APPLES.clone(), 100),
                    TokenBalance::new(BANANAS.clone(), 100),
                    TokenBalance::new(BASE_TOKEN.clone(), 200),
                ],
            ),
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*BOB),
                vec![TokenBalance::new(BANANAS.clone(), 100)],
            ),
            AddressBalance::new(
                ellipticoin::Address::Contract(ADDRESS.clone()),
                vec![TokenBalance::new(ELC.clone(), 100)],
            ),
        ];
        let mut state = HashMap::new();
        let mut api = setup(balances, &mut state);

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

        native::remove_liquidity(&mut api, APPLES.clone(), apples_in_pool).unwrap();
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
        let balances: Vec<AddressBalance> = vec![AddressBalance::new(
            ellipticoin::Address::PublicKey(*ALICE),
            vec![
                TokenBalance::new(APPLES.clone(), 1),
                TokenBalance::new(BASE_TOKEN.clone(), 1),
            ],
        )];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

        native::create_pool(&mut api, APPLES.clone(), 1 * BASE_FACTOR, BASE_FACTOR).unwrap();
        assert!(native::remove_liquidity(&mut api, APPLES.clone(), 2 * BASE_FACTOR).is_err());
    }

    #[test]
    fn test_exchange() {
        let balances: Vec<AddressBalance> = vec![
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*ALICE),
                vec![
                    TokenBalance::new(APPLES.clone(), 100),
                    TokenBalance::new(BANANAS.clone(), 100),
                    TokenBalance::new(BASE_TOKEN.clone(), 200),
                ],
            ),
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*BOB),
                vec![TokenBalance::new(APPLES.clone(), 100)],
            ),
        ];
        let mut state = HashMap::new();
        let mut api = setup(balances, &mut state);

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
    fn test_exchange_base_token() {
        let balances: Vec<AddressBalance> = vec![
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*ALICE),
                vec![
                    TokenBalance::new(APPLES.clone(), 100),
                    TokenBalance::new(BASE_TOKEN.clone(), 100),
                ],
            ),
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*BOB),
                vec![TokenBalance::new(BASE_TOKEN.clone(), 100)],
            ),
        ];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*ALICE),
                vec![
                    TokenBalance::new(APPLES.clone(), 100),
                    TokenBalance::new(BASE_TOKEN.clone(), 100),
                ],
            ),
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*BOB),
                vec![TokenBalance::new(APPLES.clone(), 100)],
            ),
        ];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*ALICE),
                vec![
                    TokenBalance::new(APPLES.clone(), 100),
                    TokenBalance::new(BANANAS.clone(), 100),
                    TokenBalance::new(BASE_TOKEN.clone(), 200),
                ],
            ),
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*BOB),
                vec![TokenBalance::new(APPLES.clone(), 100)],
            ),
        ];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

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
        let balances: Vec<AddressBalance> = vec![
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*ALICE),
                vec![
                    TokenBalance::new(APPLES.clone(), 100),
                    TokenBalance::new(BANANAS.clone(), 100),
                    TokenBalance::new(BASE_TOKEN.clone(), 200),
                ],
            ),
            AddressBalance::new(
                ellipticoin::Address::PublicKey(*BOB),
                vec![TokenBalance::new(APPLES.clone(), 100)],
            ),
        ];
        let mut state = TestState::new();
        let mut api = setup(balances, &mut state);

        api.caller = Address::PublicKey(BOB.clone());
        let exchange_res = native::exchange(
            &mut api,
            APPLES.clone(),
            BANANAS.clone(),
            100 * BASE_FACTOR,
            0,
        );

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
