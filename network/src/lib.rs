use async_std::net::SocketAddr;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::pin::Pin;
pub use async_std::sync;
use async_std::task;
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

#[derive(Clone, Debug)]
pub struct Sender {
    inner: sync::Sender<Vec<u8>>,
}

// pub async fn send<M: Clone + Serialize>(&mut self, message: M) {
// pub async fn next<T: DeserializeOwned>(&mut self) -> Result<T, serde_cbor::error::Error> {

#[derive(Debug)]
pub struct Server {
    pub private_key: Vec<u8>,
    pub bootnodes: Vec<SocketAddr>,
    pub listener: Option<TcpListener>,
    pub receiver: async_std::sync::Receiver<Vec<u8>>,
    pub sender: futures::channel::mpsc::Sender<Vec<u8>>,
}

impl Server {
    pub async fn listen(
        &mut self,
        address: SocketAddr,
        sender: async_std::sync::Sender<Vec<u8>>,
        mut receiver: futures::channel::mpsc::Receiver<Vec<u8>>,
        bootnodes: Vec<SocketAddr>,
    ) {
        let listener = TcpListener::bind(address).await.unwrap();
        let (read_sender, mut read_receiver) = async_std::sync::channel::<futures::io::WriteHalf<TcpStream>>(1);
        let (write_sender, mut write_receiver) = async_std::sync::channel::<futures::io::WriteHalf<TcpStream>>(1);
        task::spawn(async move {
            let mut streams = vec![];
            // let mut streams2 = vec![];
            for bootnode in bootnodes {
                let stream = TcpStream::connect(bootnode).await.unwrap();
                let (mut read_half, write_half) = stream.split(); 
                streams.push(write_half);
                // let reader = async_std::io::BufReader::new(read_half);
                // futures::StreamExt::fuse(reader.lines()).await;
                // let mut lines_from_server = reader.lines();
                task::spawn(async move {
                    let mut buf = vec![0u8; 4];
                    read_half.read(&mut buf).await.unwrap();
                    // sender.send(buf).await;
                    // println!("ohhh shiiiiit {:?}", std::str::from_utf8(&buf).unwrap());
                });

                // streams2.push(read_half);

            }
            let mut next_write_receiver_fused =  write_receiver.next().fuse();
            loop{
            select! {
                stream = next_write_receiver_fused => {
                    streams.push(stream.expect("no stream"));
                },
                message = receiver.next() => {
                    println!("{:?} {}", message, streams.len());
                    if let Some(message) = message {
                        for mut stream in &mut streams {
                            stream.write_all(&message.clone()).await.expect("failed to write");
                        }
                    }
                },
                complete => (),
            }}
        });

        task::spawn(async move {
            let mut incoming = listener.incoming();
            while let Some(stream) = incoming.next().await {
                let stream = stream.unwrap();
                let mut buf = vec![0u8; 4];
                let (mut read_half, write_half) = stream.split();
                read_half.read(&mut buf).await.unwrap();
                sender.send(buf).await;
                // read_sender.send(write_half).await;
                write_sender.send(write_half).await;
            }
        });
    }
}

impl Stream for Server {
    type Item = Vec<u8>;
    fn poll_next(
        mut self: Pin<&mut Self>,
        _ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::option::Option<<Self as futures::stream::Stream>::Item>> {
        async_std::sync::Receiver::poll_next(Pin::new(&mut self.receiver), _ctx)
    }
}
impl Sink<Vec<u8>> for Server {
    type Error = futures::channel::mpsc::SendError;
    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.sender.poll_ready(cx)
    }
    fn start_send(mut self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        self.sender.start_send(item)
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
    // use serde::de::DeserializeOwned;
    use serde::Deserialize;
    use serde::Serialize;
    use async_std::sync::channel;

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    pub enum Message {
        Content(String),
    }
    #[async_std::test]
    async fn it_works() {
        // let message = Message::Content("test".to_string());
        // let expected_message = message.clone();
        let alices_key = ed25519::Keypair::generate();
        let bobs_key = ed25519::Keypair::generate();
        let (s, r) = channel::<Vec<u8>>(1);
        let (s1, r1) = futures::channel::mpsc::channel::<Vec<u8>>(1);
        let mut alices_server = Server {
            private_key: alices_key.encode().clone().to_vec(),
            bootnodes: vec![],
            listener: None,
            receiver: r,
            sender: s1,
        };
        alices_server
            .listen("0.0.0.0:1234".parse().unwrap(), s, r1, vec![])
            .await;
        let (mut alices_sender, mut alices_receiver) = alices_server.split();

        let (s2, r2) = channel::<Vec<u8>>(1);
        let (s3, r3) =  futures::channel::mpsc::channel::<Vec<u8>>(1);
        let mut bobs_server = Server {
            private_key: bobs_key.encode().clone().to_vec(),
            bootnodes: vec!["0.0.0.0:1234".parse().unwrap()],
            listener: None,
            receiver: r2,
            sender: s3,
        };
        bobs_server
            .listen(
                "0.0.0.0:1235".parse().unwrap(),
                s2,
                r3,
                vec!["0.0.0.0:1234".parse().unwrap()],
            )
            .await;
        let (mut bobs_sender, mut bobs_receiver) = bobs_server.split();
        bobs_sender.send("test\n".as_bytes().to_vec()).await.unwrap();
        assert_eq!(alices_receiver.next().await.unwrap(), "test".as_bytes().to_vec());
        alices_sender.send("boom\n".as_bytes().to_vec()).await.unwrap();
        // assert_eq!(bobs_receiver.next().await.unwrap(), vec![1, 2, 3]);
    }
}
