use crate::constants::BLOCK_BROADCASTER;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<i64>,
}

pub async fn broadcaster(_req: tide::Request<()>, sender: tide::sse::Sender) -> tide::Result<()> {
    let mut block_broadcaster = BLOCK_BROADCASTER.clone();
    while let Some(event) = block_broadcaster.recv().await {
        sender
            .send("block", event.to_string(), Some(&event.to_string()))
            .await?;
    }
    Ok(())
}
