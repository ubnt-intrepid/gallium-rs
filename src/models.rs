use super::schema::{users, public_keys};
use chrono::NaiveDateTime;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct PublicKey {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub key: String,
    pub user_id: i32,
}

#[derive(Insertable)]
#[table_name = "public_keys"]
pub struct NewPublicKey<'a> {
    pub key: &'a str,
    pub user_id: i32,
}


#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[has_many(public_keys)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email_address: String,
    pub bcrypt_hash: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email_address: &'a str,
    pub bcrypt_hash: &'a str,
}
