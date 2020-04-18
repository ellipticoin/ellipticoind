use vm::zero_pad_vec;
pub const SYSTEM_ADDRESS: [u8; 32] = [0; 32];
// 3i5sXB1tP/yiftq6nl6wqLgGAdAnXml7hLuenYcln+8= in hex
pub const GENISIS_ADRESS: [u8; 32] =
    hex!("de2e6c5c1d6d3ffca27edaba9e5eb0a8b80601d0275e697b84bb9e9d87259fef");
lazy_static! {
    pub static ref TOKEN_CONTRACT: Vec<u8> =
        { [&SYSTEM_ADDRESS.to_vec(), "Ellipticoin".as_bytes()].concat() };
    pub static ref SYSTEM_CONTRACT_ADDRESS: Vec<u8> = vec![0; 32];
    pub static ref BALANCES_ENUM: Vec<u8> = vec![1];
    pub static ref CURRENT_MINER_ENUM: Vec<u8> = vec![2];
    pub static ref RANDOM_SEED_ENUM: Vec<u8> = vec![4];
    pub static ref ETHEREUM_BALANCE_ENUM: Vec<u8> = vec![5];
    pub static ref ETHEREUM_BALANCE_PREFIX: Vec<u8> = [
        zero_pad_vec(&SYSTEM_CONTRACT_ADDRESS, 255),
        ETHEREUM_BALANCE_ENUM.to_vec()
    ]
    .concat();
    pub static ref BALANCES_PREFIX: Vec<u8> =
        [SYSTEM_CONTRACT_ADDRESS.to_vec(), BALANCES_ENUM.to_vec()].concat();
    pub static ref ETHEREUM_BALANCE_KEY: Vec<u8> = [
        zero_pad_vec(&SYSTEM_CONTRACT_ADDRESS, 255),
        ETHEREUM_BALANCE_ENUM.to_vec()
    ]
    .concat();
    pub static ref RANDOM_SEED_KEY: Vec<u8> = [
        zero_pad_vec(&SYSTEM_CONTRACT_ADDRESS, 255),
        RANDOM_SEED_ENUM.to_vec()
    ]
    .concat();
    pub static ref CURRENT_MINER_KEY: Vec<u8> = [
        zero_pad_vec(&SYSTEM_CONTRACT_ADDRESS, 255),
        CURRENT_MINER_ENUM.to_vec()
    ]
    .concat();
}
