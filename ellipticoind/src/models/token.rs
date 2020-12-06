use crate::schema::tokens;
use crate::helpers::sha256;
use crate::config::get_pg_connection;
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use crate::schema::tokens::{dsl as tokens_dsl};

lazy_static! {
    pub static ref ELC_ID: i32 = {
        // tokens_dsl::tokens.select(tokens_dsl::id)
    // .filter(tokens_dsl::id_bytes.eq(sha256("ELC".as_bytes().to_vec()).to_vec()))
// .get_result(&get_pg_connection()).unwrap()
    1
    };
}

pub fn get_ellipticoin_token_id(ticker: &str, conn: &diesel::PgConnection) -> i32{
        tokens_dsl::tokens.select(tokens_dsl::id)
    .filter(tokens_dsl::id_bytes.eq(sha256(ticker.as_bytes().to_vec()).to_vec()))
.get_result(conn).unwrap()
}
#[derive(Queryable, Associations, PartialEq, Default)]
#[belongs_to(Network)]
pub struct Token {
    pub name: String,
    pub bytes_id: Vec<u8>,
    pub network_id: i32,
}


#[derive(Queryable, Associations, PartialEq, Default)]
pub struct Network {
    pub name: String,
}
