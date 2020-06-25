use crate::{
    config::HOST,
    constants::{Namespace, TOKEN_CONTRACT},
    vm::state::State,
};
use serde::{Deserialize, Serialize};
use serde_cbor::from_slice;
use std::convert::TryInto;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Miner {
    pub host: String,
    pub address: Vec<u8>,
    pub burn_per_block: u64,
    pub hash_onion_skin: Vec<u8>,
}

impl State {
    pub fn current_miner(&mut self) -> Option<Miner> {
        let miners: Vec<Miner> =
            from_slice(&self.get_storage(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]))
                .unwrap_or(vec![]);
        miners.first().map(|miner| (*miner).clone())
    }

    pub fn block_number(&mut self) -> u32 {
        let bytes = self.get_storage(&TOKEN_CONTRACT, &vec![Namespace::BlockNumber as u8]);
        if bytes.len() == 0 {
            0
        } else {
            u32::from_le_bytes(bytes[..].try_into().unwrap())
        }
    }

    pub async fn peers(&mut self) -> Vec<String> {
        let miners: Vec<Miner> = serde_cbor::from_slice(
            &self.get_storage(&TOKEN_CONTRACT, &vec![Namespace::Miners as u8]),
        )
        .unwrap();
        miners
            .iter()
            .map(|miner| miner.host.clone())
            .filter(|host| host.to_string() != *HOST)
            .collect()
    }
}
