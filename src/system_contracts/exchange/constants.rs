use crate::helpers::zero_pad_vec;
use ellipticoin::{Address, Token};
use std::convert::TryInto;

pub const FEE: u64 = 2500;

lazy_static! {
    pub static ref BASE_TOKEN: Token = Token {
        issuer: Address::PublicKey(
            base64::decode("OaKmwCWrUhdCCsIMN/ViVcu1uBF0VM3FW3Mi1z/VTNs").unwrap()[..]
                .try_into()
                .unwrap()
        ),
        token_id: zero_pad_vec(
            &hex::decode("4748b2e6DB310512Ff9085E533b6C4151ff10746").unwrap(),
            32
        )[..]
            .try_into()
            .unwrap()
    };
}
