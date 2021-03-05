use crate::{
    charge,
    constants::USD,
    contract::{self, Contract},
    pay,
    token::Token,
};
use anyhow::{anyhow, bail, Result};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{Address, DB};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OrderType {
    Sell,
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

pub struct OrderBook;

impl Contract for OrderBook {
    const NAME: contract::Name = contract::Name::OrderBook;
}

db_accessors!(OrderBook {
    orders() -> Vec<Order>;
    order_id_counter() -> u64;
});

impl OrderBook {
    pub fn create_order<D: DB>(
        db: &mut D,
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
            OrderType::Sell => {
                charge!(db, sender, token, amount)?;
            }
        }
        orders.push(order);
        Self::increment_order_id_counter(db);
        Self::set_orders(db, orders);
        Ok(())
    }

    pub fn cancel<D: DB>(db: &mut D, sender: Address, order_id: u64) -> Result<()> {
        let mut orders = Self::get_orders(db);
        let index = orders
            .iter()
            .cloned()
            .position(|order| order.id == order_id)
            .ok_or(anyhow!("Order {} not found", order_id))?;
        if orders[index].sender != sender {
            bail!("Permission denied")
        }
        orders.remove(index);
        Self::set_orders(db, orders);
        Ok(())
    }

    pub fn fill<D: DB>(db: &mut D, sender: Address, order_id: u64) -> Result<()> {
        let orders = Self::get_orders(db);
        let index = orders
            .iter()
            .cloned()
            .position(|order| order.id == order_id)
            .ok_or(anyhow!("Order {} not found", order_id))?;
        let order = orders[index].clone();
        match order.order_type {
            OrderType::Sell => {
                Token::transfer(db, sender, order.sender, order.amount, USD)?;
                pay!(db, sender, order.token, order.amount)?;
            }
        }
        Ok(())
    }

    fn increment_order_id_counter<D: DB>(db: &mut D) -> u64 {
        let order_id_counter = Self::get_order_id_counter(db) + 1;
        Self::set_order_id_counter(db, order_id_counter);
        order_id_counter
    }
}

#[cfg(test)]
mod tests {
    use super::{Order, OrderBook, OrderType};
    use crate::{order_book::USD, Token};
    use ellipticoin_test_framework::{
        constants::{
            actors::{ALICE, BOB},
            tokens::APPLES,
        },
        test_db::TestDB,
    };

    #[test]
    fn test_create_order() {
        let mut db = TestDB::new();
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
        let mut db = TestDB::new();
        Token::set_balance(&mut db, ALICE, APPLES, 1);
        OrderBook::create_order(&mut db, ALICE, OrderType::Sell, 1, APPLES, 1).unwrap();
        OrderBook::cancel(&mut db, ALICE, 0).unwrap();
        assert_eq!(OrderBook::get_orders(&mut db), vec![]);
    }

    #[test]
    fn test_fill() {
        let mut db = TestDB::new();
        Token::set_balance(&mut db, ALICE, APPLES, 1);
        Token::set_balance(&mut db, BOB, USD, 1);
        OrderBook::create_order(&mut db, ALICE, OrderType::Sell, 1, APPLES, 1).unwrap();
        OrderBook::fill(&mut db, BOB, 0).unwrap();
        assert_eq!(Token::get_balance(&mut db, ALICE, APPLES), 0);
        assert_eq!(Token::get_balance(&mut db, ALICE, USD), 1);
        assert_eq!(Token::get_balance(&mut db, BOB, APPLES), 1);
        assert_eq!(Token::get_balance(&mut db, BOB, USD), 0);
    }
}
