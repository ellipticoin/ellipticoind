use crate::system_contracts::token::constants::DAI;
use ellipticoin::Token;

pub const FEE: u64 = 3000;

lazy_static! {
    pub static ref BASE_TOKEN: Token = DAI.clone();
}
