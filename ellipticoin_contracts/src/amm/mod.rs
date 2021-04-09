mod validations;
use crate::{
    charge,
    constants::{BASE_FACTOR, FEE, LEVERAGED_BASE_TOKEN},
    contract::{self, Contract},
    helpers::proportion_of,
    pay, Token,
};
use anyhow::{anyhow, bail, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use linked_hash_set::LinkedHashSet;
use std::cmp::max;

pub struct AMM;

impl Contract for AMM {
    const NAME: contract::Name = contract::Name::AMM;
}

db_accessors!(AMM {
    balance(address: Address, token: Address) -> u64;
    total_supply(token: Address) -> u64;
    pool_supply_of_base_token(token: Address) -> u64;
    pool_supply_of_token(token: Address) -> u64;
    liquidity_providers(token: Address) -> LinkedHashSet<Address>;
});

impl AMM {
    pub fn get_underlying_pool_supply_of_base_token<B: Backend>(
        db: &mut Db<B>,
        token: Address,
    ) -> u64 {
        let pool_supply_of_base_token = Self::get_pool_supply_of_base_token(db, token);
        Token::amount_to_underlying(db, pool_supply_of_base_token, token)
    }

    pub fn create_pool<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        amount: u64,
        token: Address,
        starting_price: u64,
    ) -> Result<()> {
        let base_token_amount = proportion_of(amount, starting_price, BASE_FACTOR);
        Self::validate_pool_does_not_exist(db, token)?;
        Self::charge(db, sender, token, amount)?;
        Self::charge_base_token(db, sender, token, base_token_amount)?;
        Self::mint_liquidity(db, sender, token, amount)?;
        Ok(())
    }

    pub fn add_liquidity<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        amount: u64,
        token: Address,
    ) -> Result<()> {
        Self::validate_pool_exists(db, token)?;
        let pool_supply_of_token = Self::get_pool_supply_of_token(db, token);
        let pool_supply_of_base_token = Self::get_pool_supply_of_base_token(db, token);
        let total_supply_of_liquidity_token = Self::get_total_supply(db, token);

        Self::mint_liquidity(
            db,
            sender,
            token,
            proportion_of(
                amount,
                total_supply_of_liquidity_token,
                pool_supply_of_token,
            ),
        )?;
        Self::charge(db, sender, token, amount)?;
        Self::charge_base_token(
            db,
            sender,
            token,
            proportion_of(amount, pool_supply_of_base_token, pool_supply_of_token),
        )?;

        Ok(())
    }

    pub fn remove_liquidity<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        percentage: u64,
        token: Address,
    ) -> Result<()> {
        let liquidity_token_balance = Self::get_balance(db, sender, token);
        let total_supply_of_liquidity_token = Self::get_total_supply(db, token);
        let pool_supply_of_token = Self::get_pool_supply_of_token(db, token);
        let pool_supply_of_base_token = Self::get_pool_supply_of_base_token(db, token);
        let amount_to_burn = proportion_of(liquidity_token_balance, percentage, BASE_FACTOR);

        Self::burn_liquidity(db, sender, token, amount_to_burn)?;
        Self::pay_base_token(
            db,
            sender,
            token,
            proportion_of(
                amount_to_burn,
                pool_supply_of_base_token,
                total_supply_of_liquidity_token,
            ),
        )?;
        Self::pay(
            db,
            sender,
            token,
            proportion_of(
                amount_to_burn,
                pool_supply_of_token,
                total_supply_of_liquidity_token,
            ),
        )?;
        Ok(())
    }

    pub fn trade<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        input_amount: u64,
        input_token: Address,
        minimum_output_amount: u64,
        output_token: Address,
    ) -> Result<()> {
        charge!(db, sender, input_token, input_amount)?;
        let base_token_amount = Self::sell(db, input_token, input_amount)?;
        let output_token_amount = Self::buy(db, output_token, base_token_amount)?;
        Self::validate_slippage(minimum_output_amount, output_token_amount)?;
        pay!(db, sender, output_token, output_token_amount)?;

        Ok(())
    }

    fn sell<B: Backend>(db: &mut Db<B>, token: Address, amount: u64) -> Result<u64> {
        if token == LEVERAGED_BASE_TOKEN {
            return Ok(amount);
        };
        Self::validate_pool_exists(db, token)?;
        let output_amount = Self::calculate_output_amount(
            Self::get_pool_supply_of_token(db, token),
            Self::get_pool_supply_of_base_token(db, token),
            amount
                .checked_sub(Self::fee(amount))
                .ok_or(anyhow!("fee was greater than amount"))?,
        );
        Self::debit_pool_supply_of_base_token(db, token, output_amount)?;
        Self::credit_pool_supply_of_token(db, token, amount);
        Ok(output_amount)
    }

    fn buy<B: Backend>(db: &mut Db<B>, token: Address, amount: u64) -> Result<u64> {
        if token == LEVERAGED_BASE_TOKEN {
            return Ok(amount);
        };
        Self::validate_pool_exists(db, token)?;
        let output_amount = Self::calculate_output_amount(
            Self::get_pool_supply_of_base_token(db, token),
            Self::get_pool_supply_of_token(db, token),
            amount
                .checked_sub(Self::fee(amount))
                .ok_or(anyhow!("fee was greater than amount"))?,
        );
        Self::debit_pool_supply_of_token(db, token, output_amount)?;
        Self::credit_pool_supply_of_base_token(db, token, amount);
        Ok(output_amount)
    }

    fn calculate_output_amount(input_supply: u64, output_supply: u64, input_amount: u64) -> u64 {
        let new_input_supply = input_supply as u128 + input_amount as u128;
        let invariant = input_supply as u128 * output_supply as u128;
        let new_output_supply = (invariant as u128 / new_input_supply as u128) as u64;
        output_supply - new_output_supply
    }

    fn fee(amount: u64) -> u64 {
        max(
            ((amount as u128 * FEE as u128) / BASE_FACTOR as u128) as u64,
            1u64,
        )
    }

    fn charge<B: Backend>(
        db: &mut Db<B>,
        address: Address,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        charge!(db, address, token, amount)?;
        Self::credit_pool_supply_of_token(db, token, amount);
        Ok(())
    }

    fn charge_base_token<B: Backend>(
        db: &mut Db<B>,
        address: Address,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        charge!(db, address, LEVERAGED_BASE_TOKEN, amount)?;
        Self::credit_pool_supply_of_base_token(db, token, amount);
        Ok(())
    }

    fn pay<B: Backend>(
        db: &mut Db<B>,
        address: Address,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        Self::debit_pool_supply_of_token(db, token, amount)?;
        pay!(db, address, token, amount)?;
        Ok(())
    }

    fn pay_base_token<B: Backend>(
        db: &mut Db<B>,
        address: Address,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        Self::debit_pool_supply_of_base_token(db, token, amount)?;
        pay!(db, address, LEVERAGED_BASE_TOKEN, amount)?;
        Ok(())
    }

    fn credit_pool_supply_of_base_token<B: Backend>(db: &mut Db<B>, token: Address, amount: u64) {
        let base_token_supply = Self::get_pool_supply_of_base_token(db, token);
        Self::set_pool_supply_of_base_token(db, token, base_token_supply + amount);
    }

    fn credit_pool_supply_of_token<B: Backend>(db: &mut Db<B>, token: Address, amount: u64) {
        let token_supply = Self::get_pool_supply_of_token(db, token);
        Self::set_pool_supply_of_token(db, token, token_supply + amount);
    }

    fn debit_pool_supply_of_base_token<B: Backend>(
        db: &mut Db<B>,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        let base_token_supply = Self::get_pool_supply_of_base_token(db, token);
        if base_token_supply >= amount {
            Self::set_pool_supply_of_base_token(db, token, base_token_supply - amount);
        } else {
            bail!("Insufficient balance")
        };
        Ok(())
    }

    fn debit_pool_supply_of_token<B: Backend>(
        db: &mut Db<B>,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        let token_supply = Self::get_pool_supply_of_token(db, token);
        if token_supply >= amount {
            Self::set_pool_supply_of_token(db, token, token_supply - amount);
        } else {
            bail!("Insufficient balance")
        };
        Ok(())
    }

    pub fn mint_liquidity<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        Self::mint(db, amount, token, sender);
        let mut liquidity_providers = Self::get_liquidity_providers(db, token);
        liquidity_providers.insert(sender);
        Self::set_liquidity_providers(db, token, liquidity_providers);
        Ok(())
    }

    pub fn burn_liquidity<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        token: Address,
        amount: u64,
    ) -> Result<()> {
        Self::burn(db, amount, token, sender)?;
        if Self::get_balance(db, sender, token) == 0 {
            let mut liquidity_providers = Self::get_liquidity_providers(db, token);
            liquidity_providers.remove(&sender);
            Self::set_liquidity_providers(db, token, liquidity_providers);
        }
        Ok(())
    }

    pub fn mint<B: Backend>(db: &mut Db<B>, amount: u64, token: Address, address: Address) {
        Self::credit(db, amount, token, address);
        let total_supply = Self::get_total_supply(db, token);
        Self::set_total_supply(db, token, total_supply + amount);
    }

    pub fn burn<B: Backend>(
        db: &mut Db<B>,
        amount: u64,
        token: Address,
        address: Address,
    ) -> Result<()> {
        Self::debit(db, amount, token, address)?;
        let total_supply = Self::get_total_supply(db, token);
        Self::set_total_supply(db, token, total_supply - amount);
        Ok(())
    }

    pub fn transfer<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        recipient: Address,
        amount: u64,
        token: Address,
    ) -> Result<()> {
        Self::debit(db, amount, token, sender)?;
        Self::credit(db, amount, token, recipient);
        Ok(())
    }

    pub fn credit<B: Backend>(db: &mut Db<B>, amount: u64, token: Address, address: Address) {
        let balance = Self::get_balance(db, address, token);
        Self::set_balance(db, address, token, balance + amount)
    }

    fn debit<B: Backend>(
        db: &mut Db<B>,
        amount: u64,
        token: Address,
        address: Address,
    ) -> Result<()> {
        let balance = Self::get_balance(db, address, token);

        if amount <= balance {
            Ok(Self::set_balance(db, address, token, balance - amount))
        } else {
            bail!(
                "{} has insufficient balance of {} have {} need {}",
                hex::encode(address),
                hex::encode(token),
                balance,
                amount
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{BASE_FACTOR, LEVERAGED_BASE_TOKEN};

    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB},
            tokens::{APPLES, BANANAS},
        },
        new_db, setup,
    };

    #[test]
    fn test_create_pool() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (1, APPLES),
                    (1, LEVERAGED_BASE_TOKEN),
                ],
            },
        );

        AMM::create_pool(&mut db, ALICE, 1, APPLES, BASE_FACTOR).unwrap();

        assert_eq!(AMM::get_balance(&mut db, ALICE, APPLES), 1);
        assert_eq!(
            AMM::get_liquidity_providers(&mut db, APPLES,)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![ALICE]
        );
    }

    #[test]
    fn test_recreate_pool() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (2, APPLES),
                    (2, LEVERAGED_BASE_TOKEN),
                ],
            },
        );
        AMM::create_pool(&mut db, ALICE, 1, APPLES, BASE_FACTOR).unwrap();
        assert_eq!(
            AMM::create_pool(&mut db, ALICE, 1, APPLES, BASE_FACTOR)
                .err()
                .unwrap()
                .to_string(),
            "Pool already exisits: a000000000000000000000000000000000000000"
        );

        assert_eq!(AMM::get_balance(&mut db, ALICE, APPLES), 1);
        assert_eq!(
            AMM::get_liquidity_providers(&mut db, APPLES)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![ALICE]
        );
    }

    #[test]
    fn test_create_pool_insufficient_token_balance() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (1, APPLES),
                    (2, LEVERAGED_BASE_TOKEN),
                ],
            },
        );

        assert_eq!(
            AMM::create_pool(&mut db, ALICE, 2, APPLES, BASE_FACTOR / 2)
                .err()
                .unwrap()
                .to_string(),
            "aaa1b967f4e3d67c4946ec6816b05f0207aad9cd has insufficient balance of a000000000000000000000000000000000000000 have 1 need 2"
        );
        db.revert();

        assert_eq!(AMM::get_balance(&mut db, ALICE, APPLES), 0);
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 1);
        assert_eq!(
            AMM::get_liquidity_providers(&mut db, APPLES)
                .iter()
                .cloned()
                .collect::<Vec<Address>>()
                .len(),
            0
        );
    }
    #[test]
    fn test_create_pool_insufficient_base_token_balance() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (2, APPLES),
                    (1, LEVERAGED_BASE_TOKEN),
                ],
            },
        );

        assert_eq!(
            AMM::create_pool(&mut db, ALICE, 2, APPLES, BASE_FACTOR * 2)
                .err()
                .unwrap()
                .to_string(),
            "aaa1b967f4e3d67c4946ec6816b05f0207aad9cd has insufficient balance of 5d3a536e4d6dbd6114cc1ead35777bab948e3643 have 1 need 4"
        );
        db.revert();

        assert_eq!(AMM::get_balance(&mut db, APPLES, ALICE), 0);
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 2);
        assert_eq!(
            AMM::get_liquidity_providers(&mut db, APPLES)
                .iter()
                .cloned()
                .collect::<Vec<Address>>()
                .len(),
            0
        );
    }

    #[test]
    fn test_add_liquidity() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (2, APPLES),
                    (2, LEVERAGED_BASE_TOKEN),
                ],
            },
        );

        AMM::create_pool(&mut db, ALICE, 1, APPLES, BASE_FACTOR).unwrap();
        AMM::add_liquidity(&mut db, ALICE, 1, APPLES).unwrap();

        assert_eq!(AMM::get_balance(&mut db, ALICE, APPLES), 2);
        assert_eq!(
            AMM::get_liquidity_providers(&mut db, APPLES)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![ALICE]
        );
    }

    #[test]
    fn test_add_to_existing_liquidity() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (3 * BASE_FACTOR, APPLES),
                    (3 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
            },
        );

        AMM::create_pool(&mut db, ALICE, BASE_FACTOR, APPLES, BASE_FACTOR).unwrap();
        AMM::add_liquidity(&mut db, ALICE, BASE_FACTOR, APPLES).unwrap();
        AMM::add_liquidity(&mut db, ALICE, BASE_FACTOR, APPLES).unwrap();

        assert_eq!(AMM::get_balance(&mut db, ALICE, APPLES), 3 * BASE_FACTOR);
        assert_eq!(
            AMM::get_liquidity_providers(&mut db, APPLES)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![ALICE]
        );
    }

    #[test]
    fn test_remove_liquidity() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (3 * BASE_FACTOR, APPLES),
                    (3 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
            },
        );

        AMM::create_pool(&mut db, ALICE, BASE_FACTOR, APPLES, BASE_FACTOR).unwrap();
        AMM::add_liquidity(&mut db, ALICE, BASE_FACTOR, APPLES).unwrap();
        AMM::remove_liquidity(&mut db, ALICE, BASE_FACTOR / 2, APPLES).unwrap();

        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 2 * BASE_FACTOR);
        assert_eq!(
            Token::get_balance(&mut db, ALICE, LEVERAGED_BASE_TOKEN),
            2 * BASE_FACTOR
        );
        assert_eq!(AMM::get_balance(&mut db, ALICE, APPLES), 1 * BASE_FACTOR);
        assert_eq!(
            AMM::get_liquidity_providers(&mut db, APPLES)
                .iter()
                .cloned()
                .collect::<Vec<Address>>(),
            vec![ALICE]
        );
    }

    #[test]
    fn test_trade() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (100 * BASE_FACTOR, APPLES),
                    (100 * BASE_FACTOR, BANANAS),
                    (200 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
                BOB => vec![
                    (100 * BASE_FACTOR, BANANAS),
                ],
            },
        );
        AMM::create_pool(&mut db, ALICE, 100 * BASE_FACTOR, APPLES, BASE_FACTOR).unwrap();
        AMM::create_pool(&mut db, ALICE, 100 * BASE_FACTOR, BANANAS, BASE_FACTOR).unwrap();
        AMM::trade(&mut db, BOB, 100 * BASE_FACTOR, BANANAS, 0, APPLES).unwrap();
        assert_eq!(Token::get_balance(&mut db, BOB, APPLES), 33233234);
    }

    #[test]
    fn test_one_unit_trade() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (100 * BASE_FACTOR, APPLES),
                    (100 * BASE_FACTOR, BANANAS),
                    (200 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
                BOB => vec![
                    (1 * BASE_FACTOR, BANANAS),
                ],
            },
        );
        AMM::create_pool(&mut db, ALICE, 100 * BASE_FACTOR, APPLES, BASE_FACTOR).unwrap();
        AMM::create_pool(&mut db, ALICE, 100 * BASE_FACTOR, BANANAS, BASE_FACTOR).unwrap();
        assert_eq!(
            AMM::trade(&mut db, BOB, 1, BANANAS, 0, APPLES)
                .err()
                .unwrap()
                .to_string(),
            "fee was greater than amount"
        );
        assert_eq!(Token::get_balance(&mut db, BOB, APPLES), 0);
    }

    #[test]
    fn test_trade_base_token() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (100 * BASE_FACTOR, APPLES),
                    (100 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
                BOB => vec![
                    (100 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
            },
        );
        AMM::create_pool(
            &mut db,
            ALICE,
            100 * BASE_FACTOR,
            APPLES.clone(),
            BASE_FACTOR,
        )
        .unwrap();

        AMM::trade(
            &mut db,
            BOB,
            100 * BASE_FACTOR,
            LEVERAGED_BASE_TOKEN.clone(),
            0,
            APPLES.clone(),
        )
        .unwrap();
        assert_eq!(
            Token::get_balance(&mut db, BOB, APPLES.clone(),),
            49_924_888
        );
    }

    #[test]
    fn test_trade_for_base_token() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (100 * BASE_FACTOR, APPLES),
                    (100 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
                BOB => vec![
                    (100 * BASE_FACTOR, APPLES),
                ],
            },
        );
        AMM::create_pool(
            &mut db,
            ALICE,
            100 * BASE_FACTOR,
            APPLES.clone(),
            BASE_FACTOR,
        )
        .unwrap();
        AMM::trade(
            &mut db,
            BOB,
            100 * BASE_FACTOR,
            APPLES.clone(),
            0,
            LEVERAGED_BASE_TOKEN.clone(),
        )
        .unwrap();
        assert_eq!(
            Token::get_balance(&mut db, BOB, LEVERAGED_BASE_TOKEN.clone()),
            49_924_888
        );
    }

    #[test]
    fn test_trade_max_slippage_exceeded() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (100 * BASE_FACTOR, APPLES),
                    (100 * BASE_FACTOR, BANANAS),
                    (200 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
                BOB => vec![
                    (100 * BASE_FACTOR, APPLES),
                ],
            },
        );
        AMM::create_pool(
            &mut db,
            ALICE,
            100 * BASE_FACTOR,
            APPLES.clone(),
            BASE_FACTOR,
        )
        .unwrap();
        AMM::create_pool(
            &mut db,
            ALICE,
            100 * BASE_FACTOR,
            BANANAS.clone(),
            BASE_FACTOR,
        )
        .unwrap();

        assert_eq!(
            AMM::trade(
                &mut db,
                BOB,
                100 * BASE_FACTOR,
                APPLES.clone(),
                33_233_235,
                BANANAS.clone(),
            )
            .err()
            .unwrap()
            .to_string(),
            "Maximum slippage exceeded"
        );
    }

    #[test]
    fn test_trade_with_invariant_overflow() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (100_000 * BASE_FACTOR, APPLES),
                    (1000 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
                BOB => vec![
                    (100_000 * BASE_FACTOR, APPLES),
                ],
            },
        );

        AMM::create_pool(
            &mut db,
            ALICE,
            100_000 * BASE_FACTOR,
            APPLES.clone(),
            BASE_FACTOR / 100,
        )
        .unwrap();

        AMM::trade(
            &mut db,
            BOB,
            100 * BASE_FACTOR,
            APPLES.clone(),
            0,
            LEVERAGED_BASE_TOKEN.clone(),
        )
        .unwrap();
        assert_eq!(
            Token::get_balance(&mut db, BOB, LEVERAGED_BASE_TOKEN.clone(),),
            996_007
        );
        AMM::trade(
            &mut db,
            BOB,
            996_007,
            LEVERAGED_BASE_TOKEN.clone(),
            0,
            APPLES.clone(),
        )
        .unwrap();
        assert_eq!(
            Token::get_balance(&mut db, BOB, APPLES.clone(),),
            99_999_401_499
        );
        assert_eq!(
            Token::get_balance(&mut db, BOB, LEVERAGED_BASE_TOKEN.clone(),),
            0
        );

        AMM::remove_liquidity(&mut db, ALICE, BASE_FACTOR, APPLES).unwrap();
        let alices_apples = Token::get_balance(&mut db, ALICE, APPLES);
        let bobs_apples = Token::get_balance(&mut db, BOB, APPLES);
        assert_eq!(alices_apples + bobs_apples, 200_000 * BASE_FACTOR);
    }

    #[test]
    fn test_remove_liquidity_after_trade() {
        let mut db = new_db();
        setup(
            &mut db,
            hashmap! {
                ALICE => vec![
                    (100 * BASE_FACTOR, APPLES),
                    (100 * BASE_FACTOR, BANANAS),
                    (200 * BASE_FACTOR, LEVERAGED_BASE_TOKEN),
                ],
                BOB => vec![
                    (100 * BASE_FACTOR, BANANAS),
                ],
            },
        );
        AMM::create_pool(&mut db, ALICE, 100 * BASE_FACTOR, APPLES, BASE_FACTOR).unwrap();
        AMM::create_pool(&mut db, ALICE, 100 * BASE_FACTOR, BANANAS, BASE_FACTOR).unwrap();
        AMM::trade(&mut db, BOB, 100 * BASE_FACTOR, BANANAS, 0, APPLES).unwrap();
        AMM::remove_liquidity(&mut db, ALICE, BASE_FACTOR, APPLES).unwrap();
        let alices_apples = Token::get_balance(&mut db, ALICE, APPLES);
        let alices_base_tokens = Token::get_balance(&mut db, ALICE, LEVERAGED_BASE_TOKEN);
        assert_eq!(alices_apples, 66766766);
        assert_eq!(alices_base_tokens, 149924888);
        let bobs_apples = Token::get_balance(&mut db, BOB, APPLES);
        assert_eq!(bobs_apples, 33233234);
        assert_eq!(alices_apples + bobs_apples, 100 * BASE_FACTOR);
    }
}
