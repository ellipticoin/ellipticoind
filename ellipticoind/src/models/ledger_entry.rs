use crate::{
    models::{
        address::{Credit, Debit},
        transaction::Transaction,
        token::Token,
    },
    schema::ledger_entries,
    // schema::ledger_entries::dsl::{ledger_entries as ledger_entries_table},
};
// use crate::config::get_pg_connection;
// use diesel::{insert_into, OptionalExtension, QueryDsl};
// use crate::diesel::RunQueryDsl;

#[derive(Queryable, Associations, Insertable, PartialEq, Default)]
#[belongs_to(Transaction)]
#[belongs_to(Token)]
#[belongs_to(Debit, foreign_key = "debit_id")]
#[belongs_to(Credit, foreign_key = "credit_id")]
#[table_name = "ledger_entries"]
pub struct LedgerEntry {
    transaction_id: i32,
    token_id: i32,
    amount: i64,
    credit_id: i32,
    debit_id: i32,
}

// impl LedgerEntry {
//     pub fn insert(
//         transaction: &Transaction,
//     ) {
//         let id = insert_into(ledger_entries_table)
//             .values(&LedgerEntry::from(transaction))
//             .execute(&get_pg_connection())
//             .unwrap();
//     }    
// }
// impl From<&Transaction> for LedgerEntry {
//     fn from(transaction: &Transaction) -> Self {
//         LedgerEntry{
//             amount: match transaction.function.as_ref() {
//                 "transfer" => {
//                     transaction.arguments[1]
//                 }
//             },
//             ..Default::default()
//         }
//     }
// }
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diesel::{QueryDsl, RunQueryDsl},
        schema::ledger_entries::dsl::ledger_entries,
    };
    use crate::models::token::ELC_ID;
    use crate::{
        diesel::ExpressionMethods,
        models::{address::Address, Block},
        schema::{
            addresses::dsl as addresses_dsl, balances::dsl as balances_dsl,
            blocks::dsl as blocks_dsl, transactions::dsl as transactions_dsl,
        },
    };
    use diesel::{pg::upsert::excluded, result::Error, Connection, PgConnection};
    use crate::models::token::get_ellipticoin_token_id;

    fn get_database_url() -> String {
        dotenv::dotenv().ok();
        std::env::var("DATABASE_URL").unwrap_or("postgres://:@/ellipticoind-test".to_string())
    }
    #[test]
    fn test_new_ledger_entry() {
        let conn = PgConnection::establish(&get_database_url()).unwrap();
        let elc_id = get_ellipticoin_token_id("ELC", &conn);
        conn.test_transaction::<_, Error, _>(|| {
            let block_number = diesel::insert_into(blocks_dsl::blocks)
                .values(Block {
                    ..Default::default()
                })
                .returning(blocks_dsl::number)
                .get_result::<i32>(&conn)
                .unwrap();

            let transaction_id = diesel::insert_into(transactions_dsl::transactions)
                .values(&Transaction {
                    block_number,
                    ..Default::default()
                })
                .returning(transactions_dsl::id)
                .get_result::<i32>(&conn)
                .unwrap();
            let alice = diesel::insert_into(addresses_dsl::addresses)
                .values(Address {
                    bytes: [0u8; 32].to_vec(),
                })
                .on_conflict(addresses_dsl::id)
                .do_update()
                .set(addresses_dsl::id.eq(excluded(addresses_dsl::id)))
                .returning(addresses_dsl::id)
                .get_result::<i32>(&conn)
                .unwrap();
            let bob = diesel::insert_into(addresses_dsl::addresses)
                .values(Address {
                    bytes: [0u8; 32].to_vec(),
                })
                .on_conflict(addresses_dsl::id)
                .do_update()
                .set(addresses_dsl::id.eq(excluded(addresses_dsl::id)))
                .returning(addresses_dsl::id)
                .get_result::<i32>(&conn)
                .unwrap();
            diesel::insert_into(ledger_entries)
                .values(LedgerEntry {
                    transaction_id,
                    debit_id: alice,
                    credit_id: bob,
                    token_id: elc_id,
                    amount: 1.into(),
                    ..Default::default()
                })
                .execute(&conn)
                .unwrap();
            let bobs_balance = balances_dsl::balances
                .filter(balances_dsl::id.eq(bob))
                .select(balances_dsl::balance)
                .get_result::<i64>(&conn)?;
            assert!(bobs_balance == 1i64);
            Ok(())
        });
    }
}
