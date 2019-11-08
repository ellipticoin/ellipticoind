#![feature(async_closure)]
extern crate bytes;
extern crate mime;
extern crate rand;
extern crate serde_cbor;
extern crate tokio;

use crate::rand::Rng;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::time::Duration;
mod api;

pub async fn run(socket: SocketAddr) {
    let api = api::API::new();
    let mut api2 = api.clone();
    tokio::spawn(async move {
        mine(&mut api2).await;
    });
    api.serve(socket).await;
}

async fn mine(mut api: &mut api::API) {
    loop {
        mine_next_block(&mut api).await;
    }
}
async fn mine_next_block(api: &mut api::API) {
    api::blocks::NEXT_BLOCK_NUMBER.fetch_add(1, Ordering::Relaxed);
    let block_winner = rand::thread_rng().gen::<[u8; 32]>();
    api.broadcast(block_winner.to_vec()).await;
    let random = rand::thread_rng().gen_range(0, 5000);
    tokio::timer::delay_for(Duration::from_millis(random)).await;
}
