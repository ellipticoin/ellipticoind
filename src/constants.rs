use std::time::Duration;
lazy_static! {
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(4);
}
pub const TRANSACTION_FEE: u32 = 100;
pub const FREE_FUNCTIONS: [&'static str; 4] = [
    "reveal",
    "start_mining",
    "unlock_ether",
    "transfer_to_current_miner",
];
pub const GENESIS_STATE_PATH: &'static str = "./dist/genesis.cbor";
pub const TOKEN_WASM_PATH: &'static str = "./contracts/token/dist/token.wasm";
pub const SYSTEM_ADDRESS: [u8; 32] = [0; 32];
lazy_static! {
    pub static ref TOKEN_CONTRACT: Vec<u8> =
        [&SYSTEM_ADDRESS.to_vec(), "Ellipticoin".as_bytes()].concat();
}
pub enum Namespace {
    _Allowances,
    _Balances,
    BlockNumber,
    EthereumBalances,
    Miners,
    UnlockedEthereumBalances,
    _TotalUnlockedEthereum,
}
