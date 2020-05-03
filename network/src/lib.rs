#![recursion_limit = "256"]
pub extern crate serde;
extern crate rand;
use async_std::net::SocketAddr;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::pin::Pin;
pub use async_std::sync;
use async_std::task;
use rand::seq::SliceRandom;
use futures::channel::mpsc;
use futures::io::WriteHalf;
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
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::mem;

#[derive(Clone, Debug)]
pub struct Sender {
    inner: sync::Sender<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Protocol {
    Hello(SocketAddr),
    NewPeer(SocketAddr),
    Message(Vec<u8>),
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
            let mut length_buf = [0u8; mem::size_of::<u32>()];
            read_half.read(&mut length_buf).await.unwrap();
            let length = u32::from_le_bytes(length_buf) as usize;
            let mut buf = vec![0u8; length];
            read_half.read_exact(&mut buf).await.unwrap();
            sender.send(buf).await.unwrap();
        }
    });
}
impl<
        S: Clone + Serialize + std::marker::Send + 'static + std::marker::Sync,
        D: DeserializeOwned + std::marker::Send + 'static,
    > Server<S, D>
{
    pub fn new(private_key: Vec<u8>, socket_addr: SocketAddr, bootnodes: Vec<SocketAddr>) -> Self {
        Self {
            private_key,
            bootnodes,
            socket_addr,
            incommming_channel: mpsc::channel::<D>(1),
            outgoing_channel: mpsc::channel::<S>(1),
        }
    }
    pub async fn channel(self) -> (mpsc::Sender<S>, mpsc::Receiver<D>) {
        let socket_addr = self.socket_addr;
        let bootnodes = self.bootnodes;
        let listener = TcpListener::bind(socket_addr).await.unwrap();
        let (mut read_sender, mut read_receiver) = mpsc::unbounded();
        let (stream_sender, mut stream_receiver) = mpsc::channel::<TcpStream>(1);
        let (outgoing_sender, mut outgoing_receiver) = self.outgoing_channel;
        let (mut incommming_sender, incomming_receiver) = self.incommming_channel;
        task::spawn(async move {
            let mut streams = vec![];
            let random_bootnode = bootnodes.choose(&mut rand::thread_rng());
            if let Some(bootnode) = random_bootnode {
                let mut stream = TcpStream::connect(bootnode).await.unwrap();
                let outgoing_message_bytes = serde_cbor::to_vec(&Protocol::Hello(
                        socket_addr
                        ))
                    .unwrap();
                let length_bytes = u32::to_le_bytes(outgoing_message_bytes.len() as u32);
                use std::time::Duration;

use async_std::task;

                stream.write_all(&length_bytes).await.unwrap();
                stream.write_all(&outgoing_message_bytes).await.unwrap();
                task::sleep(Duration::from_secs(1)).await;
                handle_stream(stream, &mut streams, read_sender.clone()).await;
                // let (read_half, write_half) = stream.split();
                // streams.push(write_half);
                // spawn_read_loop(read_half, read_sender.clone()).await;

            };
            
            loop {
                let mut next_stream_receiver_fused = stream_receiver.next().fuse();
                let mut next_read_receiver_fused = read_receiver.next().fuse();
                select! {
                    stream = next_stream_receiver_fused =>{
                        if let Some(stream) = stream {
                            // broadcast_new_peer(&mut streams, stream.local_addr().unwrap()).await;
                            handle_stream(stream, &mut streams, read_sender.clone()).await;
                        }
                },
                    incommming_message = next_read_receiver_fused =>{
                        handle_incomming_message(socket_addr, &mut streams, incommming_message.unwrap(), &mut incommming_sender, &mut read_sender).await;},
                    outgoing_message = outgoing_receiver.next() =>{
                        if let Some(outgoing_message) = outgoing_message {
                        handle_outgoing_message(&mut streams, outgoing_message).await
                        }},
                    complete => (),
                }
            }
        });

        task::spawn(async move {
            listener
                .incoming()
                .map(Result::unwrap)
                .map(Ok)
                .forward(stream_sender)
                .await
                .unwrap();
        });
        (outgoing_sender, incomming_receiver)
    }
}

async fn broadcast_new_peer(
    socket_addr: SocketAddr,
    streams: &mut Vec<(SocketAddr, WriteHalf<TcpStream>)>,
    peer: SocketAddr,
) {
    for (stream_addr, stream) in streams {
        if (socket_addr.eq(stream_addr)) {
            break;
        }
        let outgoing_message_bytes = serde_cbor::to_vec(&Protocol::NewPeer(
                peer
        ))
        .unwrap();
        let length_bytes = u32::to_le_bytes(outgoing_message_bytes.len() as u32);
        stream.write_all(&length_bytes).await.unwrap();
        stream.write_all(&outgoing_message_bytes).await.unwrap();
    }
}

async fn handle_outgoing_message<
    S: Clone + Serialize + std::marker::Send + 'static + std::marker::Sync,
>(
    streams: &mut Vec<(SocketAddr, WriteHalf<TcpStream>)>,
    outgoing_message: S,
) {
    for (_, stream) in streams {
        let outgoing_message_bytes = serde_cbor::to_vec(&Protocol::Message(
            serde_cbor::to_vec(&outgoing_message).unwrap(),
        ))
        .unwrap();
        let length_bytes = u32::to_le_bytes(outgoing_message_bytes.len() as u32);
        stream.write_all(&length_bytes).await.unwrap();
        stream.write_all(&outgoing_message_bytes).await.unwrap();
    }
}

async fn handle_incomming_message<D: DeserializeOwned + std::marker::Send + 'static>(
    socket_addr: SocketAddr,
    mut streams: &mut Vec<(SocketAddr, WriteHalf<TcpStream>)>,
    incommming_message: Vec<u8>,
    incomming_sender: &mut mpsc::Sender<D>,
    read_sender: &mut mpsc::UnboundedSender<Vec<u8>>,
) {
    match serde_cbor::from_slice(&incommming_message) {
        Ok(Protocol::Message(incommming_message)) => {
            incomming_sender
                .send(serde_cbor::from_slice(&incommming_message).unwrap())
                .await
                .unwrap();
        },
        Ok(Protocol::Hello(address)) => {
            broadcast_new_peer(socket_addr, streams, address).await;
        },
        Ok(Protocol::NewPeer(address)) => {
            let stream = TcpStream::connect(address).await.unwrap();
            handle_stream(stream, &mut streams, read_sender.clone()).await;
        },
        _ => (),
    }
}

async fn handle_stream(
    stream: TcpStream,
    streams: &mut Vec<(SocketAddr, WriteHalf<TcpStream>)>,
    read_sender: mpsc::UnboundedSender<Vec<u8>>,
) {
    let addr = stream.local_addr().unwrap();
    let (read_half, write_half) = stream.split();
    streams.push((addr, write_half));
    spawn_read_loop(read_half, read_sender.clone()).await;
}

impl<
        S: Clone + Serialize + std::marker::Send + 'static,
        D: DeserializeOwned + std::marker::Send + 'static,
    > Stream for Server<S, D>
{
    type Item = D;
    fn poll_next(
        mut self: Pin<&mut Self>,
        _ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::option::Option<<Self as futures::stream::Stream>::Item>> {
        mpsc::Receiver::poll_next(Pin::new(&mut self.incommming_channel.1), _ctx)
    }
}
impl<
        S: Clone + Serialize + std::marker::Send + 'static,
        D: DeserializeOwned + std::marker::Send + 'static,
    > Sink<S> for Server<S, D>
{
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
            "127.0.0.1:1234".parse().unwrap(),
            vec![],
        );
        let (mut alices_sender, mut alices_receiver) = alices_server.channel().await;

        let bobs_server: Server<Message, Message> = Server::new(
            bobs_key.encode().clone().to_vec(),
            "127.0.0.1:1235".parse().unwrap(),
            vec!["127.0.0.1:1234".parse().unwrap()],
        );
        let (mut bobs_sender, mut bobs_receiver) = bobs_server.channel().await;
        bobs_sender.send(message.clone()).await.unwrap();
        assert_eq!(alices_receiver.next().await.unwrap(), expected_message);
        alices_sender.send(message.clone()).await.unwrap();
        assert_eq!(bobs_receiver.next().await.unwrap(), expected_message);
        let carols_key = ed25519::Keypair::generate();
        let carols_server: Server<Message, Message> = Server::new(
            carols_key.encode().clone().to_vec(),
            "127.0.0.1:1236".parse().unwrap(),
            vec!["127.0.0.1:1235".parse().unwrap()],
        );
        let (mut carols_sender, _carols_receiver) = carols_server.channel().await;
        carols_sender.send(message.clone()).await.unwrap();
        assert_eq!(alices_receiver.next().await.unwrap(), expected_message);
    }
}
