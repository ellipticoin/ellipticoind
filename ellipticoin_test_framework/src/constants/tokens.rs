use crate::constants::actors::ALICE;
use ellipticoin::{Address, Token};

lazy_static! {
    pub static ref APPLES: Token = Token {
        issuer: Address::PublicKey(*ALICE),
        id: vec![0].into()
    };
    pub static ref BANANAS: Token = Token {
        issuer: Address::PublicKey(*ALICE),
        id: vec![1].into()
    };
}
