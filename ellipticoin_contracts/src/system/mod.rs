use crate::contract::{self, Contract};
use ellipticoin_macros::db_accessors;
use ellipticoin_types::{Address, DB};

pub struct System;

impl Contract for System {
    const NAME: contract::Name = contract::Name::System;
}

db_accessors!(System {
    block_number() -> u64;
    transaction_number(address: Address) -> u64;
});

impl System {
    pub fn get_next_transaction_number<D: DB>(db: &mut D, address: Address) -> u64 {
        if Self::get_transaction_number(db, address) == 0 {
            1
        } else {
            Self::get_transaction_number(db, address) + 1
        }
    }
    pub fn increment_block_number<D: DB>(db: &mut D) {
        let block_number = Self::get_block_number(db) + 1;
        Self::set_block_number(db, block_number);
    }

    pub fn increment_transaction_number<D: DB>(db: &mut D, address: Address) {
        let transaction_number = System::get_next_transaction_number(db, address);
        Self::set_transaction_number(db, address, transaction_number);
    }
}
