use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Arc;
use async_std::sync::Mutex;
use async_std::task;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::sink::SinkExt;
use futures::StreamExt;
use serde::Serialize;
use std::net::SocketAddr;
use tungstenite::Message;

#[derive(Clone, Default)]
pub struct Websocket {
    pub senders: Arc<Mutex<Vec<UnboundedSender<Message>>>>,
}
impl Websocket {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub async fn send<M: Clone + Serialize>(&mut self, message: M) {
        for sender in self.senders.lock().await.iter_mut() {
            let _ = sender
                .send(Message::binary(serde_cbor::to_vec(&message).unwrap()))
                .await;
        }
    }
    pub async fn bind(self, socket: SocketAddr) {
        let try_socket = TcpListener::bind(&socket).await;
        let listener = try_socket.expect("Failed to bind");

        while let Ok((stream, _)) = listener.accept().await {
            let (sender, receiver) = futures::channel::mpsc::unbounded();
            task::spawn(accept_connection(stream, receiver));
            self.senders.lock().await.push(sender);
        }
    }
}

async fn accept_connection(stream: TcpStream, receiver: UnboundedReceiver<Message>) {
    if let Ok(ws_stream) = async_tungstenite::accept_async(stream).await {
        let (write, _read) = ws_stream.split();
        let _ = receiver.map(Ok).forward(write).await;
    }
}
