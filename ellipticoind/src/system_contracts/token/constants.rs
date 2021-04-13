use crate::system_contracts::bridge;
use ellipticoin::Token;

lazy_static! {
    pub static ref ELC: Token = Token {
        id: "ELC".as_bytes().to_vec().into(),
        issuer: crate::system_contracts::token::Address::Contract("Ellipticoin".to_string())
    };
    pub static ref DAI: Token = bridge::token(
        hex::decode("6b175474e89094c44da98b954eedeac495271d0f")
            .unwrap()
            .into()
    );
    pub static ref BTC: Token = bridge::token(
        hex::decode("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2")
            .unwrap()
            .into()
    );
    pub static ref ETH: Token = bridge::token(
        hex::decode("eb4c2781e4eba804ce9a9803c67d0893436bb27d")
            .unwrap()
            .into()
    );
}
