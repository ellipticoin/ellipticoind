#![recursion_limit="256"]
pub extern crate serde;
use async_std::net::SocketAddr;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::pin::Pin;
pub use async_std::sync;
use serde::de::DeserializeOwned;
use async_std::task;
use serde::Serialize;
use futures::prelude::*;
pub use futures::{
    future,
    future::FutureExt,
    pin_mut, select,
    sink::SinkExt,
    stream::StreamExt,
    task::{Context, Poll},
    AsyncRead, AsyncWrite, Sink, Stream,
};
use futures::channel::mpsc;

#[derive(Clone, Debug)]
pub struct Sender {
    inner: sync::Sender<Vec<u8>>,
}

#[derive(Debug)]
pub struct Server<S: Clone + Serialize + std::marker::Send, D: DeserializeOwned> {
    pub private_key: Vec<u8>,
    pub socket_addr: SocketAddr,
    pub bootnodes: Vec<SocketAddr>,
    pub incommming_channel: (mpsc::Sender<D>, mpsc::Receiver<D>),
    pub outgoing_channel: (mpsc::Sender<S>, mpsc::Receiver<S>),
}

pub async fn spawn_read_loop(
    mut read_half: futures::io::ReadHalf<TcpStream>,
    mut sender: mpsc::UnboundedSender<Vec<u8>>,
) {
    task::spawn(async move {
        loop {
            let mut length_buf = [0u8; 4];
            read_half.read(&mut length_buf).await.unwrap();
            let length = u32::from_le_bytes(length_buf) as usize;
            let mut buf = vec![0u8; length];
            read_half.read_exact(&mut buf).await.unwrap();
            sender.send(buf).await.unwrap();
        }
    });
}
impl<S: Clone + Serialize + std::marker::Send + 'static + std::marker::Sync, D: DeserializeOwned + std::marker::Send +'static> Server<S, D> {
    pub fn new(
    private_key: Vec<u8>,
    socket_addr: SocketAddr,
    bootnodes: Vec<SocketAddr>,
    ) -> Self {
        Self {
            private_key,
            bootnodes,
            socket_addr,
            incommming_channel: mpsc::channel::<D>(1),
            outgoing_channel: mpsc::channel::<S>(1),
        }
    }
    pub async fn channel(
        self,
    ) ->
(mpsc::Sender<S>, mpsc::Receiver<D>)
    {
        let socket_addr = self.socket_addr;
        let bootnodes = self.bootnodes;
        let listener = TcpListener::bind(socket_addr).await.unwrap();
        let (read_sender, mut read_receiver) = mpsc::unbounded();
        let (stream_sender, mut stream_receiver) = mpsc::channel::<TcpStream>(1);
        let (outgoing_sender, mut outgoing_receiver)  = self.outgoing_channel;
        let (mut incommming_sender, incomming_receiver)  = self.incommming_channel;
        task::spawn(async move {
            let mut streams = vec![];
            for bootnode in bootnodes {
                if let Ok(stream) = TcpStream::connect(bootnode).await {
                    let (read_half, write_half) = stream.split();
                    streams.push(write_half);
                    spawn_read_loop(read_half, read_sender.clone()).await;
                }
            }
            loop {
                let mut next_stream_receiver_fused = stream_receiver.next().fuse();
                let mut next_read_receiver_fused = read_receiver.next().fuse();
                select! {
                    stream = next_stream_receiver_fused => {
                        let (mut read_half, write_half) = stream.expect("1").split();
                        spawn_read_loop(read_half,read_sender.clone()).await;
                        streams.push(write_half);
                    },
                    incommming_message = next_read_receiver_fused => {
                        if let Some(incommming_message) = incommming_message {
                            incommming_sender.send(serde_cbor::from_slice(&incommming_message).unwrap()).await.unwrap();
                        }
                    },
                    outgoing_message = outgoing_receiver.next() => {
                        if let Some(outgoing_message) = outgoing_message {
                            for mut stream in &mut streams {
                                let outgoing_message_bytes = serde_cbor::to_vec(&outgoing_message).unwrap();
                                let length_bytes = u32::to_le_bytes(outgoing_message_bytes.len() as u32); 
                                stream.write_all(&length_bytes).await.expect("failed to write");
                                stream.write_all(&outgoing_message_bytes).await.expect("failed to write");
                            }
                        }
                    },
                    complete => (),
                }
            }
        });

        task::spawn(async move {
            listener.incoming()
            .map(Result::unwrap)
            .map(Ok)
            .forward(stream_sender).await.unwrap();
        });
        (outgoing_sender, incomming_receiver)
    }
}

impl<S: Clone + Serialize+ std::marker::Send +'static, D: DeserializeOwned+ std::marker::Send +'static> Stream for Server<S, D> {
    type Item = D;
    fn poll_next(
        mut self: Pin<&mut Self>,
        _ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::option::Option<<Self as futures::stream::Stream>::Item>> {
        mpsc::Receiver::poll_next(Pin::new(&mut self.incommming_channel.1), _ctx)
    }
}
impl<S: Clone + Serialize+ std::marker::Send +'static, D: DeserializeOwned+ std::marker::Send +'static> Sink<S> for Server<S, D> {
    type Error = futures::channel::mpsc::SendError;
    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.outgoing_channel.0.poll_ready(cx)
    }
    fn start_send(mut self: Pin<&mut Self>, item: S) -> Result<(), Self::Error> {
        self.outgoing_channel.0.start_send(item)
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::ed25519;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    pub enum Message {
        Content(String),
    }
    #[async_std::test]
    async fn it_works() {
        let message = Message::Content("test".to_string());
        let expected_message = message.clone();
        let alices_key = ed25519::Keypair::generate();
        let bobs_key = ed25519::Keypair::generate();
        let alices_server: Server<Message, Message> = Server::new(
            alices_key.encode().clone().to_vec(),
            "0.0.0.0:1234".parse().unwrap(),
            vec![],
        );
        let (mut alices_sender, mut alices_receiver) = alices_server
            .channel()
            .await;

        let bobs_server: Server<Message, Message> = Server::new(
            bobs_key.encode().clone().to_vec(),
                "0.0.0.0:1235".parse().unwrap(),
            vec!["0.0.0.0:1234".parse().unwrap()],
        );
        let (mut bobs_sender, mut bobs_receiver) = bobs_server
            .channel()
            .await;
        bobs_sender
            .send(message.clone())
            .await
            .unwrap();
        assert_eq!(
            alices_receiver.next().await.unwrap(),
            expected_message
        );
        alices_sender
            .send(message)
            .await
            .unwrap();
        assert_eq!(
            bobs_receiver.next().await.unwrap(),
            expected_message
        );
    }
}
