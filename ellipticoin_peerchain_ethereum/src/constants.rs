use ellipticoin_contracts::constants::BASE_FACTOR;
use hex_literal::hex;
use lazy_static::lazy_static;
use std::{collections::HashMap, env};
use ellipticoin_contracts::constants::{BTC, ETH, USD};

lazy_static! {
    pub static ref ELLIPTICOIN_DECIMALS: usize = BASE_FACTOR.to_string().len() - 1;
    pub static ref DECIMALS: HashMap<[u8; 20], u64> = {
        let mut decimals = HashMap::new();
        decimals.insert(BTC, 8);
        decimals.insert(ETH_ADDRESS, 18);
        // decimals.insert(USD, 18);
        decimals.insert(USD, 8);
        decimals
    };
    pub static ref WEB3_URL: String = env::var("WEB3_URL").expect("WEB3_URL not set");
}

pub static REDEEM_TIMEOUT: u64 = 30;
pub const TRANSFER_TOPIC: [u8; 32] =
    hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
pub const REDEEM_TOPIC: [u8; 32] =
    hex!("ff051e185ca4ab867487cbb2112ad9dcf4b6e45ec93c6c83fe371bfd126d1da6");
pub const RECEIVED_ETH_TOPIC: [u8; 32] =
    hex!("4103257eaac983ca79a70d28f90dfc4fa16b619bb0c17ee7cab0d4034c279624");

pub const TOKENS: [[u8; 20]; 3] = [BTC, ETH, USD];
// pub const BRIDGE_ADDRESS: [u8; 20] = hex!("E55faDE7825Ad88581507C51c9f1b33827AaE5E8");
pub const BRIDGE_ADDRESS: [u8; 20] = hex!("6f246D6B8C0cca9298C685D02dFDA3A666e6e067");
pub const ETH_ADDRESS: [u8; 20] = hex!("0000000000000000000000000000000000000000");