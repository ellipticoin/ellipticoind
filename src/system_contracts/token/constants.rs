use crate::system_contracts::bridge;
use ellipticoin::Token;
use std::convert::TryInto;

lazy_static! {
    pub static ref DAI: Token = bridge::token(
        [
            [0; 12].to_vec(),
            hex::decode("6b175474e89094c44da98b954eedeac495271d0f").unwrap()
        ]
        .concat()[..]
            .try_into()
            .unwrap()
    );
    pub static ref BTC: Token = bridge::token(
        [
            [0; 12].to_vec(),
            hex::decode("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap()
        ]
        .concat()[..]
            .try_into()
            .unwrap()
    );
    pub static ref ETH: Token = bridge::token(
        [
            [0; 12].to_vec(),
            hex::decode("eb4c2781e4eba804ce9a9803c67d0893436bb27d").unwrap()
        ]
        .concat()[..]
            .try_into()
            .unwrap()
    );
}
