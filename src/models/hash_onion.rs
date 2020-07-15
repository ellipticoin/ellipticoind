use crate::{
    diesel::{QueryDsl, RunQueryDsl},
    helpers::sha256,
    schema::hash_onion::dsl::*,
};
use diesel::{
    dsl::*,
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
pub use diesel_migrations::revert_latest_migration;
use indicatif::ProgressBar;
use rand::Rng;

use crate::schema::hash_onion;
use std::env;

use diesel::{dsl::sql_query, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    Associations,
    PartialEq,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[primary_key(layer)]
#[table_name = "hash_onion"]
pub struct HashOnion {
    pub layer: Vec<u8>,
}

impl HashOnion {
    pub fn peel(pg_db: &PooledConnection<ConnectionManager<PgConnection>>) -> Vec<u8> {
        let skin = hash_onion
            .select(layer)
            .order(id.desc())
            .first::<Vec<u8>>(pg_db)
            .unwrap();
        sql_query(
            "delete from hash_onion where id in (
        select id from hash_onion order by id desc limit 1
    )",
        )
        .execute(pg_db)
        .unwrap();
        skin
    }

    pub fn generate(db: &PooledConnection<ConnectionManager<PgConnection>>) {
        let hash_onion_size = env::var(&"HASH_ONION_SIZE")
            .map(|hash_onion_size| hash_onion_size.parse().unwrap())
            .unwrap_or(31 * 24 * 60 * 60);
        let sql_query_size = 65534;
        let center: Vec<u8> = rand::thread_rng()
            .sample_iter(&rand::distributions::Standard)
            .take(32)
            .collect();
        let mut onion = vec![center];

        println!("Generating Hash Onion");
        let pb = ProgressBar::new(hash_onion_size);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
                .progress_chars("=> "),
        );
        for _ in (0..hash_onion_size).step_by(sql_query_size) {
            pb.inc(sql_query_size as u64);
            for _ in 1..(sql_query_size) {
                onion.push(sha256(onion.last().unwrap().to_vec()).to_vec());
            }
            let values: Vec<HashOnion> = onion
                .iter()
                .map(|hash| HashOnion {
                    layer: hash.to_vec(),
                })
                .collect();
            let query = insert_into(hash_onion).values(&values);
            query.execute(db).unwrap();
            onion = vec![onion.last().unwrap().to_vec()];
        }
        pb.finish();
    }
}
