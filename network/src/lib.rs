extern crate ed25519_dalek;
extern crate rand;
pub extern crate serde;
use async_std::{
    net::{SocketAddr, TcpListener, TcpStream},
    task,
};
pub use futures::{
    channel::mpsc,
    future,
    future::FutureExt,
    io::WriteHalf,
    pin_mut,
    prelude::*,
    select,
    sink::SinkExt,
    stream::StreamExt,
    task::{Context, Poll},
    AsyncRead, AsyncWrite, Sink, Stream,
};
use rand::seq::SliceRandom;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::mem;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Protocol {
    Join(SocketAddr),
    Peers(Vec<SocketAddr>),
    NewPeer(SocketAddr),
    Message(Vec<u8>),
}

#[derive(Debug)]
pub struct Server<S: Clone + Serialize + std::marker::Send, D: DeserializeOwned> {
    pub private_key: Vec<u8>,
    pub socket_addr: SocketAddr,
    pub external_socket_addr: SocketAddr,
    pub bootnodes: Vec<SocketAddr>,
    pub incomming_channel: (mpsc::Sender<D>, mpsc::Receiver<D>),
    pub outgoing_channel: (mpsc::Sender<S>, mpsc::Receiver<S>),
}

async fn send(stream: &mut WriteHalf<TcpStream>, message: &Protocol) {
    let outgoing_message_bytes = serde_cbor::to_vec(&message).unwrap();
    let length_bytes = u32::to_le_bytes(outgoing_message_bytes.len() as u32);
    stream.write_all(&length_bytes).await.unwrap();
    stream.write_all(&outgoing_message_bytes).await.unwrap();
}

pub async fn receive(read_half: &mut futures::io::ReadHalf<TcpStream>) -> Vec<u8> {
    let mut length_buf = [0u8; mem::size_of::<u32>()];
    read_half.read(&mut length_buf).await.unwrap();
    let length = u32::from_le_bytes(length_buf) as usize;
    let mut buf = vec![0u8; length];
    read_half.read_exact(&mut buf).await.unwrap();
    buf
}

pub async fn spawn_read_loop(
    mut read_half: futures::io::ReadHalf<TcpStream>,
    mut sender: mpsc::UnboundedSender<Vec<u8>>,
) {
    task::spawn(async move {
        loop {
            sender.send(receive(&mut read_half).await).await.unwrap();
        }
    });
}

impl<
        S: Clone + Serialize + std::marker::Send + 'static + std::marker::Sync,
        D: DeserializeOwned + std::marker::Send + 'static,
    > Server<S, D>
{
    pub fn new(
        private_key: Vec<u8>,
        socket_addr: SocketAddr,
        external_socket_addr: SocketAddr,
        bootnodes: Vec<SocketAddr>,
    ) -> Self {
        Self {
            private_key,
            bootnodes,
            external_socket_addr,
            socket_addr,
            incomming_channel: mpsc::channel::<D>(1),
            outgoing_channel: mpsc::channel::<S>(1),
        }
    }

    pub async fn channel(self) -> (mpsc::Sender<S>, mpsc::Receiver<D>) {
        let socket_addr = self.socket_addr;
        let external_socket_addr = self.external_socket_addr;
        let bootnodes = self.bootnodes;
        let listener = TcpListener::bind(socket_addr).await.unwrap();
        let (read_sender, mut read_receiver) = mpsc::unbounded();
        let (stream_sender, mut stream_receiver) = mpsc::channel::<TcpStream>(1);
        let (outgoing_sender, mut outgoing_receiver) = self.outgoing_channel;
        let (mut incomming_sender, incomming_receiver) = self.incomming_channel;
        let mut streams = vec![];
        let random_bootnode = bootnodes.choose(&mut rand::thread_rng());
        if let Some(bootnode) = random_bootnode {
            let stream = TcpStream::connect(bootnode).await.unwrap();
            let addr = stream.peer_addr().unwrap();
            let (mut read_half, mut write_half) = stream.split();
            send(&mut write_half, &Protocol::Join(external_socket_addr)).await;
            streams.push((addr, write_half));
            let message = receive(&mut read_half).await;
            if let Ok(Protocol::Peers(peers)) = serde_cbor::from_slice(&message) {
                connect_to_peers(
                    external_socket_addr,
                    &mut streams,
                    peers,
                    read_sender.clone(),
                )
                .await;
                spawn_read_loop(read_half, read_sender.clone()).await;
            }
        };
        task::spawn(async move {
            loop {
                let mut next_stream_receiver_fused = stream_receiver.next().fuse();
                let mut next_read_receiver_fused = read_receiver.next().fuse();
                select! {
                    stream = next_stream_receiver_fused =>{
                        if let Some(stream) = stream {
                            handle_incomming_stream(stream, &mut streams, read_sender.clone()).await;
                        }
                },
                    incomming_message = next_read_receiver_fused =>{
                        handle_incomming_message(incomming_message.unwrap(), &mut incomming_sender).await;},
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

async fn connect_to_peers(
    external_socket_addr: SocketAddr,
    streams: &mut Vec<(SocketAddr, WriteHalf<TcpStream>)>,
    peers: Vec<SocketAddr>,
    read_sender: mpsc::UnboundedSender<Vec<u8>>,
) {
    for peer in peers {
        let stream = TcpStream::connect(peer).await.unwrap();
        let (read_half2, mut write_half2) = stream.split();
        send(&mut write_half2, &Protocol::Join(external_socket_addr)).await;
        spawn_read_loop(read_half2, read_sender.clone()).await;
        streams.push((peer, write_half2));
    }
}

async fn handle_outgoing_message<
    S: Clone + Serialize + std::marker::Send + 'static + std::marker::Sync,
>(
    streams: &mut Vec<(SocketAddr, WriteHalf<TcpStream>)>,
    outgoing_message: S,
) {
    for (_, stream) in streams {
        let message = Protocol::Message(serde_cbor::to_vec(&outgoing_message).unwrap());
        send(stream, &message).await;
    }
}

async fn handle_incomming_message<D: DeserializeOwned + std::marker::Send + 'static>(
    incomming_message: Vec<u8>,
    incomming_sender: &mut mpsc::Sender<D>,
) {
    match serde_cbor::from_slice(&incomming_message) {
        Ok(Protocol::Message(incomming_message)) => {
            incomming_sender
                .send(serde_cbor::from_slice(&incomming_message).unwrap())
                .await
                .unwrap();
        }
        _ => (),
    }
}

async fn handle_incomming_stream(
    stream: TcpStream,
    streams: &mut Vec<(SocketAddr, WriteHalf<TcpStream>)>,
    read_sender: mpsc::UnboundedSender<Vec<u8>>,
) {
    let addr = stream.peer_addr().unwrap();
    let (mut read_half, mut write_half) = stream.split();
    let message = receive(&mut read_half).await;
    if let Ok(Protocol::Join(_)) = serde_cbor::from_slice(&message) {
        let peers = streams
            .iter()
            .map(|(socket_addr, _)| socket_addr.clone())
            .collect();
        send(&mut write_half, &Protocol::Peers(peers)).await;
        streams.push((addr, write_half));
        spawn_read_loop(read_half, read_sender.clone()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Keypair;
    use rand::rngs::OsRng;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    pub enum Message {
        Content(String),
    }
    #[async_std::test]
    async fn it_works() {
        let mut csprng = OsRng {};
        let message = Message::Content("test".to_string());
        let expected_message = message.clone();
        let alices_key = Keypair::generate(&mut csprng);
        let bobs_key = Keypair::generate(&mut csprng);
        let alices_server: Server<Message, Message> = Server::new(
            alices_key.secret.to_bytes().to_vec(),
            "0.0.0.0:1234".parse().unwrap(),
            "0.0.0.0:1234".parse().unwrap(),
            vec![],
        );
        let (mut alices_sender, mut alices_receiver) = alices_server.channel().await;

        let bobs_server: Server<Message, Message> = Server::new(
            bobs_key.secret.to_bytes().to_vec(),
            "127.0.0.1:1235".parse().unwrap(),
            "127.0.0.1:1235".parse().unwrap(),
            vec!["127.0.0.1:1234".parse().unwrap()],
        );
        let (mut bobs_sender, mut bobs_receiver) = bobs_server.channel().await;
        bobs_sender.send(message.clone()).await.unwrap();
        assert_eq!(alices_receiver.next().await.unwrap(), expected_message);
        alices_sender.send(message.clone()).await.unwrap();
        assert_eq!(bobs_receiver.next().await.unwrap(), expected_message);
        let carols_key = Keypair::generate(&mut csprng);
        let carols_server: Server<Message, Message> = Server::new(
            carols_key.secret.to_bytes().to_vec(),
            "127.0.0.1:1236".parse().unwrap(),
            "127.0.0.1:1236".parse().unwrap(),
            vec!["127.0.0.1:1235".parse().unwrap()],
        );
        let (mut carols_sender, _carols_receiver) = carols_server.channel().await;
        carols_sender.send(message.clone()).await.unwrap();
        assert_eq!(alices_receiver.next().await.unwrap(), expected_message);
    }
}
