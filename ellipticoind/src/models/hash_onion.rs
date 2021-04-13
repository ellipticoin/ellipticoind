use crate::{
    config::signing_key,
    diesel::{QueryDsl, RunQueryDsl},
    helpers::sha256,
    models::get_pg_connection,
    schema::{hash_onion, hash_onion::dsl::*},
};
use diesel::{
    dsl::{sql_query, *},
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
pub use diesel_migrations::revert_latest_migration;
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::env;

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
            .expect("No layers left on hash onion");
        sql_query(
            "delete from hash_onion where id in (
        select id from hash_onion order by id desc limit 1
    )",
        )
        .execute(pg_db)
        .unwrap();
        skin
    }

    pub fn skip(pg_db: &PooledConnection<ConnectionManager<PgConnection>>, number: usize) {
        sql_query(format!(
            "delete from hash_onion where id in (
        select id from hash_onion order by id desc limit {}
    )",
            number
        ))
        .execute(pg_db)
        .unwrap();
    }

    pub async fn generate() {
        let pg_db = get_pg_connection();
        let hash_onion_size = env::var(&"HASH_ONION_SIZE")
            .map(|hash_onion_size| hash_onion_size.parse().unwrap())
            .unwrap_or(31 * 24 * 60 * 60);
        let sql_query_size = 65534;
        let mut center = sha256(<[u8; 32]>::from(signing_key()).to_vec()).to_vec();
        println!("Generating Hash Onion");
        let pb = ProgressBar::new(hash_onion_size);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
                .progress_chars("=> "),
        );
        for chunk in (0..hash_onion_size)
            .collect::<Vec<_>>()
            .chunks(sql_query_size)
        {
            pb.inc(chunk.len() as u64);
            let mut onion: Vec<Vec<u8>> = vec![];
            for _ in chunk {
                center = sha256(center.clone().to_vec().clone()).to_vec();
                onion.push(center.clone());
            }
            let values: Vec<HashOnion> = onion
                .iter()
                .map(|hash| HashOnion {
                    layer: hash.to_vec(),
                })
                .collect();
            let query = insert_into(hash_onion).values(&values);
            query.execute(&pg_db).unwrap();
        }
        pb.finish();
    }
}
