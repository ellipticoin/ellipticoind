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
    let pb = ProgressBar::new(*HASH_ONION_SIZE as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
            .progress_chars("=> "),
    );
    let mut onion = vec![*PRIVATE_KEY];
    for layer in 1..(*HASH_ONION_SIZE) {
        if layer % 10000 == 0 {
            pb.inc(10000);
        }
        onion.push(sha256(onion.last().unwrap().to_vec()));
    }
    *ONION.lock().await = onion;
    pb.finish();
}
pub async fn peel() -> [u8; 32] {
    ONION.lock().await.pop().expect("No onion layers left")
}
