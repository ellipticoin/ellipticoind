use crate::{
    models::Transaction,
    system_contracts,
    system_contracts::{
        exchange::{
            burn_liquidity, constants::BASE_TOKEN, debit_pool_supply_of_base_token,
            debit_pool_supply_of_token, get_pool_supply_of_base_token, get_pool_supply_of_token,
            liquidity_token,
        },
        token,
    },
};
use ellipticoin::{pay, Address, Token};
use serde_cbor::Value;
// use system_contracts::exchange::CONTRACT_NAME;
use wasm_rpc::error::Error;

pub async fn run<API: ellipticoin::API>(api: &mut API, transaction: &mut Transaction) {
    fix_spelling_errors(transaction);
    if (0..1_721_122_i32).contains(&transaction.block_number)
        && transaction.function == "remove_liquidity"
    {
        run_remove_liquidity(api, transaction);
    } else {
        // system_contracts::run(api, TransactionRequest::from(transaction.clone()));
    }
}

pub fn fix_spelling_errors(mut transaction: &mut Transaction) {
    if (0..343866_i32).contains(&transaction.block_number) {
        transaction.function = match transaction.function.as_str() {
            "add_liqidity" => "add_liquidity".to_string(),
            "remove_liqidity" => "remove_liquidity".to_string(),
            function => function.to_string(),
        };
    }
}

pub fn run_remove_liquidity<API: ellipticoin::API>(api: &mut API, transaction: &mut Transaction) {
    let arguments = serde_cbor::from_slice::<Vec<Value>>(&transaction.arguments).unwrap();
    let token = serde_cbor::value::from_value::<Token>(arguments[0].clone()).unwrap();
    let amount = serde_cbor::value::from_value(arguments[1].clone()).unwrap();
    if remove_liquidity(api, token, amount).is_ok() {
        ellipticoin::API::commit(api);
    } else {
        ellipticoin::API::revert(api);
    }
}

pub fn remove_liquidity<API: ellipticoin::API>(
    api: &mut API,
    token: Token,
    amount: u64,
) -> Result<(), Box<Error>> {
    let pool_supply_of_token = get_pool_supply_of_token(api, token.clone());
    let pool_supply_of_base_token = get_pool_supply_of_base_token(api, token.clone());
    let total_suply_of_liquidity_token =
        token::get_total_supply(api, liquidity_token(token.clone()));
    burn_liquidity(
        api,
        token.clone(),
        (total_suply_of_liquidity_token as u128 * amount as u128 / pool_supply_of_token as u128)
            as u64,
    )?;
    debit_pool_supply_of_base_token(
        api,
        token.clone(),
        pool_supply_of_base_token * amount / pool_supply_of_token,
    )?;
    // pay!(
    //     api,
    //     BASE_TOKEN.clone(),
    //     api.caller(),
    //     pool_supply_of_base_token * amount / pool_supply_of_token
    // )?;

    debit_pool_supply_of_token(api, token.clone(), amount)?;
    // pay!(api, token, api.caller(), amount)?;

    Ok(())
}
