use indicatif::ProgressBar;
use std::fs::File;
use std::path::Path;
use std::io::Read;
use vm::rocksdb::ops::Open;

pub async fn initialize_rocks_db(path: &str) -> vm::rocksdb::DB {
    if Path::new(path).exists() {
        vm::rocksdb::DB::open_default(path).unwrap()
    } else {
        let db = vm::rocksdb::DB::open_default(path).unwrap();
        let mut file = File::open("dist/ethereum-balances-9858734.bin").unwrap();
        let metadata = std::fs::metadata("dist/ethereum-balances-9858734.bin").unwrap();
        let pb = ProgressBar::new(metadata.len() / 24);
        println!("Importing Ethereum Balances");
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
                .progress_chars("=> "),
        );
        let mut batch = rocksdb::WriteBatch::default();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        let mut i = 0;
        for chunk in buffer.chunks(24) {
            batch.put(
                [vec![0;32], vec![5], chunk[0..20].to_vec()].concat()
                ,
                chunk[20..24].to_vec()).unwrap();
            if i % 1000 == 0 {
                pb.inc(1000);
            }
            i += 1
        }
        pb.finish();
        println!("Writing Ethereum balances to storage...");
        db.write(batch).unwrap();
        db
    }
}
