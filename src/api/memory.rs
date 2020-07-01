use super::{helpers::base64_param, State};
use crate::api::helpers::proxy_get;
use crate::{api::helpers::to_cbor_response, config::public_key, VM_STATE};
use tide::{Response, Result};

pub async fn show(req: tide::Request<State>) -> Result<Response> {
    let contract_name: String = req.param("contract_name")?;
    let contract_owner_bytes = base64_param(&req, "contract_owner")?;
    let contract_address = [
        contract_owner_bytes.clone(),
        contract_name.as_bytes().to_vec(),
    ]
    .concat();
    let key_bytes = base64_param(&req, "key")?;

    let current_miner = {
        let mut vm_state = VM_STATE.lock().await;
        vm_state.current_miner().unwrap()
    };
    if current_miner.address.eq(&public_key()) {
        let mut vm_state = VM_STATE.lock().await;
        let value = vm_state.get_memory(&contract_address, &key_bytes);
        Ok(to_cbor_response(&value))
    } else {
        proxy_get(&req, current_miner.host).await
    }
}
