use std::time::Duration;
lazy_static! {
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(4);
}
lazy_static! {
    pub static ref TOKEN_CONTRACT: String = "Ellipticoin".to_string();
}
