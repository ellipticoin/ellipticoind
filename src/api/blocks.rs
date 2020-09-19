use super::State;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<i64>,
}

pub async fn broadcaster(req: tide::Request<State>, sender: tide::sse::Sender) -> tide::Result<()> {
    let mut new_block_broadcaster = req.state().new_block_broacaster.clone();
    while let Some(event) = new_block_broadcaster.recv().await {
        sender
            .send("block", event.to_string(), Some(&event.to_string()))
            .await?;
    }
    Ok(())
}
