#[macro_use]
extern crate lazy_static;
pub use async_std::sync;
use async_std::{sync::channel, task};
pub use futures::{
    future,
    future::FutureExt,
    pin_mut, select,
    sink::SinkExt,
    stream::StreamExt,
    task::{Context, Poll},
    AsyncRead, AsyncWrite, Sink, Stream,
};
use libp2p::identity::ed25519;
pub use libp2p::identity::Keypair;
use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    swarm::NetworkBehaviourEventProcess,
    Multiaddr, NetworkBehaviour, PeerId, Swarm,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;

lazy_static! {
    static ref OUTGOING_SENDER: futures::lock::Mutex<HashMap<PeerId, sync::Sender<Vec<u8>>>> = {
        let m = HashMap::new();
        futures::lock::Mutex::new(m)
    };
    static ref OUTGOING_RECIEIVER: futures::lock::Mutex<HashMap<PeerId, sync::Receiver<Vec<u8>>>> = {
        let m = HashMap::new();
        futures::lock::Mutex::new(m)
    };
    static ref INCOMMING_SENDER: futures::lock::Mutex<HashMap<PeerId, sync::Sender<Vec<u8>>>> = {
        let m = HashMap::new();
        futures::lock::Mutex::new(m)
    };
    static ref INCOMMING_RECIEIVER: futures::lock::Mutex<HashMap<PeerId, sync::Receiver<Vec<u8>>>> = {
        let m = HashMap::new();
        futures::lock::Mutex::new(m)
    };
}

#[derive(Clone, Debug)]
pub struct Sender {
    inner: sync::Sender<Vec<u8>>,
}

impl Sender {
    pub async fn send<M: Clone + Serialize>(&mut self, message: M) {
        self.inner
            .send(serde_cbor::to_vec(&message.clone()).unwrap())
            .await
    }
}

#[derive(Clone, Debug)]
pub struct Receiver {
    inner: sync::Receiver<Vec<u8>>,
}

impl Receiver {
    pub async fn next<T: DeserializeOwned>(&mut self) -> Result<T, serde_cbor::error::Error> {
        let bytes = self.inner.next().await.unwrap();
        serde_cbor::from_slice(&bytes)
    }
}

#[derive(Clone, Debug)]
pub struct Server {
    peer_id: PeerId,
    pub private_key: Vec<u8>,
    pub address: SocketAddr,
    pub peers: Vec<(SocketAddr, Vec<u8>)>,
    receiver: Option<sync::Receiver<Vec<u8>>>,
}

#[derive(NetworkBehaviour)]
struct Network<TSubstream: AsyncRead + AsyncWrite> {
    floodsub: Floodsub<TSubstream>,
}

impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<FloodsubEvent>
    for Network<TSubstream>
{
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            task::block_on(async {
                let mut outgoing = OUTGOING_SENDER.lock().await;
                let tx = outgoing.get_mut(&self.floodsub.local_peer_id).unwrap();
                tx.send(message.data.clone()).await;
            });
        }
    }
}

fn to_multiaddr(address: SocketAddr) -> Multiaddr {
    format!("/ip4/{}/tcp/{}", address.ip(), address.port())
        .parse()
        .unwrap()
}

impl Server {
    pub async fn new(
        private_key: Vec<u8>,
        address: SocketAddr,
        peers: Vec<(SocketAddr, Vec<u8>)>,
    ) -> Self {
        let keypair = libp2p::identity::Keypair::Ed25519(
            ed25519::Keypair::decode(&mut private_key.clone()).unwrap(),
        );
        let peer_id = PeerId::from(keypair.public());

        Self {
            peer_id,
            address,
            private_key,
            peers,
            receiver: None,
        }
    }

    pub async fn listen(&mut self) {
        let keypair = libp2p::identity::Keypair::Ed25519(
            ed25519::Keypair::decode(&mut self.private_key.clone()).unwrap(),
        );
        let (sockets, public_keys): (Vec<_>, Vec<_>) = self.peers.iter().cloned().unzip();
        let transport = libp2p::build_development_transport(keypair).unwrap();
        let floodsub_topic = floodsub::TopicBuilder::new("chat").build();

        let mut swarm = {
            let mut behaviour = Network {
                floodsub: Floodsub::new(self.peer_id.clone()),
            };

            behaviour.floodsub.subscribe(floodsub_topic.clone());
            for public_key in public_keys {
                behaviour.floodsub.add_node_to_partial_view(
                    libp2p::identity::PublicKey::Ed25519(
                        ed25519::PublicKey::decode(&public_key).unwrap(),
                    )
                    .into(),
                );
            }
            Swarm::new(transport, behaviour, self.peer_id.clone())
        };
        for socket in sockets {
            println!("Connecting to {:?}", to_multiaddr(socket));
            Swarm::dial_addr(&mut swarm, to_multiaddr(socket)).expect("failed to dial");
        }
        println!("Listening on {}", self.address);
        println!("Peer Id: {}", self.peer_id);
        Swarm::listen_on(&mut swarm, to_multiaddr(self.address)).unwrap();

        let mut receivers = INCOMMING_RECIEIVER.lock().await;
        let receiver = receivers.get_mut(&self.peer_id).unwrap();
        future::poll_fn(
            move |cx: &mut Context| -> std::task::Poll<Result<(), std::io::Error>> {
                loop {
                    match receiver.poll_next_unpin(cx) {
                        Poll::Ready(Some(line)) => swarm.floodsub.publish(&floodsub_topic, line),
                        Poll::Ready(None) => break,
                        Poll::Pending => break,
                    }
                }
                loop {
                    match swarm.poll_next_unpin(cx) {
                        Poll::Ready(Some(event)) => println!("{:?}", event),
                        Poll::Ready(None) => return Poll::Ready(Ok(())),
                        Poll::Pending => break,
                    }
                }
                Poll::Pending
            },
        )
        .await
        .unwrap()
    }

    pub async fn channel(&mut self) -> (Sender, Receiver) {
        let (outgoing_sender, outgoing_receiver): (sync::Sender<Vec<u8>>, sync::Receiver<Vec<u8>>) =
            channel(1);
        let (incommming_sender, incommming_receiver): (
            sync::Sender<Vec<u8>>,
            sync::Receiver<Vec<u8>>,
        ) = channel(1);
        OUTGOING_SENDER
            .lock()
            .await
            .insert(self.peer_id.clone(), outgoing_sender);
        let mut receivers = INCOMMING_RECIEIVER.lock().await;
        receivers.insert(self.peer_id.clone(), incommming_receiver);
        (
            Sender {
                inner: incommming_sender,
            },
            Receiver {
                inner: outgoing_receiver,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::net::{Ipv4Addr, SocketAddrV4};

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
        let mut alice = Server::new(
            alices_key.encode().clone().to_vec(),
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234).into(),
            vec![],
        )
        .await;
        let (mut alice_sender, _alice_receiver) = alice.channel().await;
        task::spawn(async move {
            alice.listen().await;
        });
        let mut bob: Server = Server::new(
            bobs_key.encode().clone().to_vec(),
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1235).into(),
            vec![(
                (SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1234)).into(),
                alices_key.public().encode().to_vec(),
            )],
        )
        .await;
        let (_bob_sender, mut bob_receiver) = bob.channel().await;
        task::spawn(async move {
            bob.listen().await;
        });
        task::spawn(async move {
            alice_sender.send(message).await;
        });

        let message_received = bob_receiver.next::<Message>().await.unwrap();
        assert_eq!(message_received, expected_message);
    }
}
