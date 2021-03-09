use async_std::sync::{Arc, Mutex};
use std::collections::HashMap;

lazy_static! {
    pub static ref IN_MEMORY_STATE: async_std::sync::Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}
