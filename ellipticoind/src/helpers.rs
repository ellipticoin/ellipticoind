use async_std::{future::Future, prelude::FutureExt as asyncStdFutureExt, task::sleep};
use futures::future::FutureExt;

use std::time::Duration;

pub async fn run_for<F>(duration: Duration, f: F)
where
    F: Future<Output = ()>,
{
    sleep(duration)
        .join(f)
        .map(|_| ())
        .race(sleep(duration))
        .await;
}
