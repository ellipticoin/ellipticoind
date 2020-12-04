use crate::models::address::{Credit, Debit};
use crate::models::transaction::Transaction;
use crate::schema::ledger_entries;

#[derive(Queryable, Associations, Insertable, PartialEq, Default)]
#[belongs_to(Transaction)]
#[belongs_to(Debit, foreign_key = "debit_id")]
#[belongs_to(Credit, foreign_key = "credit_id")]
#[table_name = "ledger_entries"]
pub struct LedgerEntry {
    transaction_id: i32,
    amount: bigdecimal::BigDecimal,
    credit_id: i32,
    debit_id: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diesel::RunQueryDsl;
    use crate::diesel::QueryDsl;
    use crate::schema::ledger_entries::dsl::ledger_entries;

    use crate::diesel::ExpressionMethods;
    use crate::models::address::Address;
    use crate::models::Block;
    use crate::schema::addresses::dsl as addresses_dsl;
    use crate::schema::blocks::dsl as blocks_dsl;
    use crate::schema::transactions::dsl as transactions_dsl;
    use crate::schema::balances::dsl as balances_dsl;
    use diesel::pg::upsert::excluded;
    use diesel::result::Error;
    use diesel::Connection;
    use diesel::PgConnection;

    fn get_database_url() -> String {
        dotenv::dotenv().ok();
        std::env::var("DATABASE_URL").unwrap_or("postgres://:@/ellipticoind-test".to_string())
    }
    #[test]
    fn test_new_ledger_entry() {
        let conn = PgConnection::establish(&get_database_url()).unwrap();
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
                    amount: 1.into(),
                    ..Default::default()
                })
                .execute(&conn)
                .unwrap();
let bobs_balance = balances_dsl::balances
    .filter(balances_dsl::id.eq(bob))
    .select(balances_dsl::balance)
    .get_result::<bigdecimal::BigDecimal>(&conn).expect("boom");
//     println!("{:?}", bobs_balance);
    assert!(bobs_balance == 1.into());
            Ok(())
        });


    }
}
