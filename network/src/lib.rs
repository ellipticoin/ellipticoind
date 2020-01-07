#[macro_use]
extern crate lazy_static;
pub use async_std::sync::{Receiver, Sender};
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
use futures_timer::Delay;
use libp2p::identity::ed25519;
pub use libp2p::identity::Keypair;
use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    swarm::NetworkBehaviourEventProcess,
    Multiaddr, NetworkBehaviour, PeerId, Swarm,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

lazy_static! {
    static ref OUTGOING_SENDER: futures::lock::Mutex<HashMap<PeerId, Sender<Vec<u8>>>> = {
        let m = HashMap::new();
        futures::lock::Mutex::new(m)
    };
    static ref OUTGOING_RECIEIVER: futures::lock::Mutex<HashMap<PeerId, Receiver<Vec<u8>>>> = {
        let m = HashMap::new();
        futures::lock::Mutex::new(m)
    };
}

#[derive(Clone)]
pub struct Server {
    peer_id: PeerId,
    pub private_key: Vec<u8>,
    pub address: SocketAddr,
    pub peers: Vec<(SocketAddr, Vec<u8>)>,
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
        println!("{:?}", peer_id);

        Self {
            peer_id,
            address,
            private_key,
            peers,
        }
    }

    pub async fn listen(&mut self, mut receiver: Receiver<Vec<u8>>) {
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
        Swarm::listen_on(&mut swarm, to_multiaddr(self.address)).unwrap();
        let mut listening = false;
    //     task::block_on::<_, Result<(), ()>>(future::poll_fn(move |cx: &mut Context| {
    //         loop {
    //             if let Poll::Ready(Some(message)) = &receiver.poll_next_unpin(cx) {
    //                 swarm
    //                     .floodsub
    //                     .publish(&floodsub_topic, message.to_vec());
    //             } else {
    //                 break;
    // }
    //         }
    //         loop {
    //             match swarm.poll_next_unpin(cx) {
    //                 Poll::Ready(Some(event)) => println!("{:?}", event),
    //                 Poll::Ready(None) => break,
    //                 Poll::Pending => {
    //                     if !listening {
    //                         if let Some(a) = Swarm::listeners(&swarm).next() {
    //                             println!("Listening on {:?}", a);
    //                             listening = true;
    //                         }
    //                     }
    //                     break
    //                 }
    //             }
    //         }
    //         Poll::Pending
    //     }));

        loop {
            let receiver_fused = receiver.next().fuse();
            let swarm_fused = swarm.next().fuse();
            pin_mut!(receiver_fused, swarm_fused);
            select! {
                maybe_message = receiver_fused => {
                    if let Some(message) = maybe_message {
                        swarm.floodsub.publish(&floodsub_topic, message);
                    }
                },
                _ = swarm_fused => (),
            };
        }
    }

    pub async fn receiver(&self) -> Receiver<Vec<u8>> {
        let (sender, receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(1);
        OUTGOING_SENDER
            .lock()
            .await
            .insert(self.peer_id.clone(), sender);
        receiver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_timer::Delay;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::time::Duration;

    #[async_std::test]
    async fn it_works() {
        let (alice_sender, alice_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(1);
        let (_bob_sender, bob_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(1);
        let message = vec![1, 2, 3];
        let alices_key = ed25519::Keypair::generate();
        let bobs_key = ed25519::Keypair::generate();
        let mut alice = Server::new(
            alices_key.encode().clone().to_vec(),
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234).into(),
            vec![],
        )
        .await;
        task::spawn(async move {
            alice.listen(alice_receiver).await;
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

        let mut bob_incomming_receiver = bob.receiver().await;
        task::spawn(async move {
            bob.listen(bob_receiver).await;
        });
        Delay::new(Duration::from_millis(500)).await;
        task::spawn(async move {
            alice_sender.send(vec![1, 2, 3]).await;
        });

        let message_received = bob_incomming_receiver.next().await.unwrap();
        assert_eq!(message_received, message);
    }
}
