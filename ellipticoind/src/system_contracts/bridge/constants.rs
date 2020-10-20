use std::convert::TryInto;

lazy_static! {
    pub static ref SIGNERS: Vec<[u8; 32]> =
        vec![
            base64::decode("OaKmwCWrUhdCCsIMN/ViVcu1uBF0VM3FW3Mi1z/VTNs").unwrap()[..]
                .try_into()
                .unwrap()
        ];
}
