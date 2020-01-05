#![feature(async_closure)]
#[macro_use]
extern crate lazy_static;
use async_std::task;
use async_std::sync::channel;
pub use async_std::sync::{Sender, Receiver};
pub use futures::{
    future,
    sink::SinkExt,
    stream::StreamExt,
    task::{Context, Poll},
    AsyncRead, AsyncWrite, Sink, Stream,
};
use futures_timer::Delay;
pub use libp2p::identity::Keypair;
use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    Multiaddr, NetworkBehaviour, PeerId, Swarm,
};
use std::net::SocketAddr;
use std::time::Duration;
use std::{collections::HashMap, error::Error, pin::Pin};

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
pub struct Server<T> {
    peer_id: PeerId,
    pub sender: Sender<T>,
}

#[derive(NetworkBehaviour)]
struct Network<TSubstream: AsyncRead + AsyncWrite> {
    floodsub: Floodsub<TSubstream>,
    mdns: Mdns<TSubstream>,
}

impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<MdnsEvent>
    for Network<TSubstream>
{
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
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

impl<T: Clone + Into<Vec<u8>> + std::marker::Send + 'static> Server<T> {
    pub async fn new(
        keypair: Keypair,
        address: SocketAddr,
        peers: Vec<SocketAddr>,
    ) -> Result<Self, Box<dyn Error>> {
        let peer_id = PeerId::from(keypair.public());
        let transport = libp2p::build_development_transport(keypair)?;
        let floodsub_topic = floodsub::TopicBuilder::new("chat").build();

        let mut swarm = {
            let mdns = task::block_on(Mdns::new())?;
            let mut behaviour = Network {
                floodsub: Floodsub::new(peer_id.clone()),
                mdns,
            };

            behaviour.floodsub.subscribe(floodsub_topic.clone());
            Swarm::new(transport, behaviour, peer_id.clone())
        };
        for peer in peers {
            println!("Connecting to {:?}", to_multiaddr(peer));
            Swarm::dial_addr(&mut swarm, to_multiaddr(peer)).unwrap();
        }
        Swarm::listen_on(&mut swarm, to_multiaddr(address))?;

        let (sender, mut incomming_receiver): (Sender<T>, Receiver<T>) = channel(1);
        let (outgoing_sender, outgoing_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
            channel(1);
        OUTGOING_SENDER
            .lock()
            .await
            .insert(peer_id.clone(), outgoing_sender);
        OUTGOING_RECIEIVER
            .lock()
            .await
            .insert(peer_id.clone(), outgoing_receiver);
        let (mut listening_sender, mut listening_receiver) = channel::<()>(1);
        let mut listening = false;
        task::spawn::<_, Result<(), ()>>(future::poll_fn(move |cx: &mut Context| {
            loop {
                if let Poll::Ready(Some(message)) = &incomming_receiver.poll_next_unpin(cx) {
                    swarm
                        .floodsub
                        .publish(&floodsub_topic, message.clone().into());
                } else {
                    break;
                }
            }
            loop {
                match swarm.poll_next_unpin(cx) {
                    Poll::Ready(Some(event)) => println!("{:?}", event),
                    Poll::Ready(None) => return Poll::Ready(Ok(())),
                    Poll::Pending => {
                        if !listening {
                            if let Some(a) = Swarm::listeners(&swarm).next() {
                                println!("Listening on {:?}", a);
                                listening = true;
                                task::block_on(async {
                                    listening_sender.send(()).await;
                                });
                            }
                        }
                        break;
                    }
                }
            }
            Poll::Pending
        }));
        listening_receiver.next().await;
        Ok(Self { sender, peer_id })
    }

    pub async fn send(
        self,
        item: T,
    ) {
        self.sender.send(item).await;
    }
}

// impl<T> Sink<T> for Server<T> {
//     type Error = std::io::Error;
//     fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
//         Ok(())
//     }
//     fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
//         self.sender.send(item)
//     }
//
//     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }
//     fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }
// }

impl Stream for Server<Vec<u8>> {
    type Item = Vec<u8>;
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
        task::block_on(async {
            let mut outgoing = OUTGOING_RECIEIVER.lock().await;
            let tx = outgoing.get_mut(&self.get_mut().peer_id).unwrap();
            task::block_on(async { std::task::Poll::Ready(tx.next().await) })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::sink::SinkExt;
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[async_std::test]
    async fn it_works() {
        let message = vec![1, 2, 3];
        let alices_key = Keypair::generate_ed25519();
        let bobs_key = Keypair::generate_ed25519();
        let mut alice = Server::new(
            alices_key,
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1234).into(),
            vec![],
        )
        .await
        .unwrap();
        task::spawn(async {
            let mut bob = Server::new(
                bobs_key,
                SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1235).into(),
                vec![SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1234).into()],
            )
            .await
            .unwrap();
            let actual_message = bob.next().await.unwrap();
            assert_eq!(actual_message, vec![1,2,3]);
        });
        alice.send(message.clone()).await;
    }
}
