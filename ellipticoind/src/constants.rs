pub const SYSTEM_ADDRESS: [u8; 32] = [0; 32];

lazy_static! {
    pub static ref SYSTEM_CONTRACT: Vec<u8> =
        { [&SYSTEM_ADDRESS.to_vec(), "System".as_bytes()].concat() };
}
