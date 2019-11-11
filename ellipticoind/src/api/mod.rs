use futures::{FutureExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};
#[derive(Clone)]
pub struct API {
    pub users: Arc<Mutex<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>,
    pub redis: vm::redis::Client,
}
pub mod blocks;
pub mod memory;
pub mod routes;
pub mod transactions;
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

impl API {
    pub fn new(redis: vm::redis::Client) -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            redis,
        }
    }

    pub async fn serve(self, socket: SocketAddr) {
        warp::serve(routes::routes(self)).run(socket).await;
    }

    pub async fn _broadcast<V: Clone + Into<Vec<u8>>>(&mut self, message: V) {
        for (&_uid, tx) in self.users.lock().unwrap().iter_mut() {
            tx.try_send(Ok(Message::binary(message.clone()))).unwrap();
        }
    }

    async fn user_connected(&mut self, ws: WebSocket) {
        let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
        let (user_ws_tx, user_ws_rx) = ws.split();
        let (tx, rx) = mpsc::unbounded_channel();
        warp::spawn(rx.forward(user_ws_tx).map(|_result| {}));
        self.users.lock().unwrap().insert(my_id, tx);
        let mut api = self.clone();
        user_ws_rx
            .for_each(async move |_msg| ())
            .then(async move |result| {
                api.user_disconnected(my_id);
                Ok::<(), ()>(result)
            })
            .await
            .unwrap();
    }

    fn user_disconnected(&mut self, my_id: usize) {
        self.users.lock().unwrap().remove(&my_id);
    }
}
