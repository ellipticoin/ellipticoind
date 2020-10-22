use crate::constants::WEB_SOCKET_BROADCASTER;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<i64>,
}

pub async fn broadcaster(_req: tide::Request<()>, sender: tide::sse::Sender) -> tide::Result<()> {
    let mut web_socket_broadcaster = WEB_SOCKET_BROADCASTER.clone();
    while let Some((block_number, current_miner)) = web_socket_broadcaster.recv().await {
        sender
            .send("block", current_miner, Some(&block_number.to_string()))
            .await?;
    }
    Ok(())
}
