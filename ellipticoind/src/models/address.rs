use crate::schema::addresses;

#[derive(Queryable, Insertable, Associations, PartialEq, Default)]
#[table_name = "addresses"]
pub struct Address {
    pub bytes: Vec<u8>,
}

pub struct Debit(pub Address);
pub struct Credit(pub Address);
