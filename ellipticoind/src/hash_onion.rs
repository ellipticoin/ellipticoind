use crate::db::get_hash_onion_layers_left;
use crate::{
    config::{HASH_ONION_SIZE, PRIVATE_KEY},
    crypto::sha256,
};
use async_std::sync::{Arc, Mutex};
use indicatif::ProgressBar;

lazy_static! {
    pub static ref ONION: async_std::sync::Arc<Mutex<Vec<[u8; 32]>>> = Arc::new(Mutex::new(vec![]));
}

pub async fn generate() {
    println!("Generating Hash Onion");
    let hash_onion_layers_left = get_hash_onion_layers_left()
        .await
        .unwrap_or(*HASH_ONION_SIZE as u64);
    let pb = ProgressBar::new(hash_onion_layers_left - 1);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
            .progress_chars("=> "),
    );
    let mut onion = vec![*PRIVATE_KEY];
    for layer in 0..(hash_onion_layers_left) {
        if layer % 10000 == 0 {
            pb.inc(10000);
        }
        onion.push(sha256(onion.last().unwrap().to_vec()));
    }
    *ONION.lock().await = onion;
    pb.finish();
}

pub async fn layers_left() -> usize {
    ONION.lock().await.len()
}

pub async fn peel() -> [u8; 32] {
    ONION.lock().await.pop().expect("No onion layers left")
}
