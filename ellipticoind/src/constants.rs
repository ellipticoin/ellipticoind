pub const SYSTEM_ADDRESS: [u8; 32] = [0; 32];

lazy_static! {
    pub static ref TOKEN_CONTRACT: Vec<u8> =
        { [&SYSTEM_ADDRESS.to_vec(), "Ellipticoin".as_bytes()].concat() };
}
