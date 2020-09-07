use std::time::Duration;
lazy_static! {
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(4);
}
pub const SYSTEM_ADDRESS: [u8; 32] = [0; 32];
lazy_static! {
    pub static ref TOKEN_CONTRACT: ([u8; 32], String) = (SYSTEM_ADDRESS, "Ellipticoin".to_string());
}
