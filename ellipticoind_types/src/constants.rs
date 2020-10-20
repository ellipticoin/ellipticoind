use types::{Address, Token};

lazy_static! {
    pub static ref ELC: Token = Token {
        issuer: Address::Contract("Ellipticoin".to_string()),
        id: "ELC".as_bytes().to_vec().into()
    };
}
