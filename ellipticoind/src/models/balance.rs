#[derive(Queryable, Associations)]
#[table_name = "balances"]
pub struct Balance {
    id: i32,
    amount: bigdecimal::BigDecimal,
}
