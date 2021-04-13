use crate::{
    charge,
    constants::{BASE_FACTOR, BASE_TOKEN},
    contract::{self, Contract},
    pay,
    token::Token,
};
use anyhow::{anyhow, bail, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{
    db::{Backend, Db},
    Address,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OrderType {
    Sell,
    Buy,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Order {
    pub id: u64,
    pub order_type: OrderType,
    pub sender: Address,
    pub token: Address,
    pub amount: u64,
    pub price: u64,
}

impl Order {
    pub fn get_underlying_price<B: Backend>(&self, db: &mut Db<B>) -> u64 {
        Token::underlying_to_amount(db, self.price, self.token)
    }

    pub fn get_underlying_amount<B: Backend>(&self, db: &mut Db<B>) -> u64 {
        Token::amount_to_underlying(db, self.amount, self.token)
    }
}

pub struct OrderBook;

impl Contract for OrderBook {
    const NAME: contract::Name = contract::Name::OrderBook;
}

db_accessors!(OrderBook {
    orders() -> Vec<Order>;
    order_id_counter() -> u64;
});

impl OrderBook {
    pub fn create_order<B: Backend>(
        db: &mut Db<B>,
        sender: Address,
        order_type: OrderType,
        amount: u64,
        token: Address,
        price: u64,
    ) -> Result<()> {
        let mut orders = Self::get_orders(db);
        let order = Order {
            id: Self::get_order_id_counter(db),
            order_type,
            sender,
            amount,
            token,
            price,
        };
        match order.order_type {
            OrderType::Buy => {
                charge!(db, sender, BASE_TOKEN, amount * price / BASE_FACTOR)?;
            }
            OrderType::Sell => {
                charge!(db, sender, token, amount)?;
            }
        }
        orders.push(order);
        Self::increment_order_id_counter(db);
        Self::set_orders(db, orders);
        Ok(())
    }

    pub fn cancel<B: Backend>(db: &mut Db<B>, sender: Address, order_id: u64) -> Result<()> {
        let mut orders = Self::get_orders(db);
        let index = orders
            .iter()
            .cloned()
            .position(|order| order.id == order_id)
            .ok_or(anyhow!("Order {} not found", order_id))?;
        if orders[index].sender != sender {
            bail!("Permission denied")
        }
        match orders[index].order_type {
            OrderType::Buy => {
                pay!(db, sender, BASE_TOKEN, orders[index].amount)?;
            }
            OrderType::Sell => {
                pay!(db, sender, orders[index].token, orders[index].amount)?;
            }
        }
        orders.remove(index);
        Self::set_orders(db, orders);
        Ok(())
    }

    pub fn fill<B: Backend>(db: &mut Db<B>, sender: Address, order_id: u64) -> Result<()> {
        let mut orders = Self::get_orders(db);
        let index = orders
            .iter()
            .cloned()
            .position(|order| order.id == order_id)
            .ok_or(anyhow!("Order {} not found", order_id))?;
        let order = orders[index].clone();
        match order.order_type {
            OrderType::Buy => {
                Token::transfer(db, sender, order.sender, order.amount, order.token)?;
                pay!(
                    db,
                    sender,
                    BASE_TOKEN,
                    order.amount * order.price / BASE_FACTOR
                )?;
            }
            OrderType::Sell => {
                Token::transfer(
                    db,
                    sender,
                    order.sender,
                    order.amount * order.price / BASE_FACTOR,
                    BASE_TOKEN,
                )?;
                pay!(db, sender, order.token, order.amount)?;
            }
        }

        orders.remove(index);
        Self::set_orders(db, orders);
        Ok(())
    }

    fn increment_order_id_counter<B: Backend>(db: &mut Db<B>) -> u64 {
        let order_id_counter = Self::get_order_id_counter(db) + 1;
        Self::set_order_id_counter(db, order_id_counter);
        order_id_counter
    }
}

#[cfg(test)]
mod tests {
    use super::{Order, OrderBook, OrderType};
    use crate::{constants::BASE_FACTOR, order_book::BASE_TOKEN, Token};
    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB},
            tokens::APPLES,
        },
        new_db,
    };

    #[test]
    fn test_create_order() {
        let mut db = new_db();
        Token::set_balance(&mut db, ALICE, APPLES, 1);
        OrderBook::create_order(&mut db, ALICE, OrderType::Sell, 1, APPLES, 1).unwrap();
        assert_eq!(
            OrderBook::get_orders(&mut db)[0],
            Order {
                id: 0,
                order_type: OrderType::Sell,
                token: APPLES,
                amount: 1,
                sender: ALICE,
                price: 1
            }
        );
    }

    #[test]
    fn test_cancel() {
        let mut db = new_db();
        Token::set_balance(&mut db, ALICE, APPLES, 1);
        OrderBook::create_order(&mut db, ALICE, OrderType::Sell, 1, APPLES, 1).unwrap();
        OrderBook::cancel(&mut db, ALICE, 0).unwrap();
        assert_eq!(OrderBook::get_orders(&mut db), vec![]);
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 1);
    }

    #[test]
    fn test_fill_sell() {
        let mut db = new_db();
        Token::set_balance(&mut db, ALICE, APPLES, 1);
        Token::set_balance(&mut db, BOB, BASE_TOKEN, 1);
        OrderBook::create_order(&mut db, ALICE, OrderType::Sell, 1, APPLES, BASE_FACTOR).unwrap();
        OrderBook::fill(&mut db, BOB, 0).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 0);
        assert_eq!(Token::get_balance(&mut db, ALICE, BASE_TOKEN), 1);
        assert_eq!(Token::get_balance(&mut db, BOB, APPLES), 1);
        assert_eq!(Token::get_balance(&mut db, BOB, BASE_TOKEN), 0);
    }

    #[test]
    fn test_fill_buy() {
        let mut db = new_db();
        Token::set_balance(&mut db, ALICE, BASE_TOKEN, 1);
        Token::set_balance(&mut db, BOB, APPLES, 1);
        OrderBook::create_order(&mut db, ALICE, OrderType::Buy, 1, APPLES, BASE_FACTOR).unwrap();
        OrderBook::fill(&mut db, BOB, 0).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 1);
        assert_eq!(Token::get_balance(&mut db, ALICE, BASE_TOKEN), 0);
        assert_eq!(Token::get_balance(&mut db, BOB, APPLES), 0);
        assert_eq!(Token::get_balance(&mut db, BOB, BASE_TOKEN), 1);
    }
}
