use crate::{
    models::verification_key,
    system_contracts::{
        api::InMemoryAPI,
        ellipticoin::{Miner, State},
    },
};
use async_std::sync::{Arc, Mutex};
use std::collections::HashMap;

lazy_static! {
    pub static ref IN_MEMORY_STATE: async_std::sync::Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub async fn get_state() -> State {
    let mut state = IN_MEMORY_STATE.lock().await;
    let mut api = InMemoryAPI::new(&mut state, None);
    let miners = crate::system_contracts::ellipticoin::get_miners(&mut api);
    let block_number = crate::system_contracts::ellipticoin::get_block_number(&mut api);
    State {
        miners,
        block_number,
    }
}
pub async fn is_mining() -> bool {
    let mut state = IN_MEMORY_STATE.lock().await;
    let mut api = InMemoryAPI::new(&mut state, None);
    let miners = crate::system_contracts::ellipticoin::get_miners(&mut api);
    miners
        .iter()
        .any(|miner| miner.address == verification_key())
}

pub async fn current_miner() -> Miner {
    let mut state = IN_MEMORY_STATE.lock().await;
    let mut api = InMemoryAPI::new(&mut state, None);
    let miners = crate::system_contracts::ellipticoin::get_miners(&mut api);
    miners.first().unwrap().clone()
}
