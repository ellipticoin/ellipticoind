use ellipticoin_types::Address;
use std::time::Duration;

pub const BTC: Address = hex!("eb4c2781e4eba804ce9a9803c67d0893436bb27d");
// pub const BTC: Address = hex!("804d9Dc7363593CcFeedbF685d76EE8f0fD844cC");
pub const ELC: Address = hex!("0000000000000000000000000000000000000001");
pub const ETH: Address = hex!("0000000000000000000000000000000000000000");
pub const USD: Address = hex!("6d7f0754ffeb405d23c51ce938289d4835be3b14");
// pub const USD: Address = hex!("6b175474e89094c44da98b954eedeac495271d0f");
// pub const USD: Address = hex!("5596ac7380a934802e782e0ff6471d642e488674");
pub const BASE_FACTOR: u64 = 1_000_000;
pub const FEE: u64 = 3_000;
pub const BASE_TOKEN: Address = USD;
pub const INCENTIVISED_POOLS: [Address; 2] = [BTC, ETH];
pub const TOKENS: [Address; 4] = [BTC, ELC, ETH, USD];
pub const MINER_ALLOW_LIST: [Address; 2] = [hex!("0113713f91dd6a7c179a038e66e5919a9a0a9d1d"), hex!("418b993b7d17b45937ef4f69a06a3433cd30b5ce")];

lazy_static! {
    pub static ref BLOCK_TIME: Duration = Duration::from_secs(4);
}
