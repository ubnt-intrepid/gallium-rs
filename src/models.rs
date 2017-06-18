use super::schema::public_keys;
use chrono::NaiveDateTime;

#[derive(Debug, Queryable)]
pub struct PublicKey {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub key: String,
}

#[derive(Insertable)]
#[table_name = "public_keys"]
pub struct NewPublicKey<'a> {
    pub key: &'a str,
}
