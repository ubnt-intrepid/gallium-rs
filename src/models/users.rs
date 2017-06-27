use chrono::NaiveDateTime;
use schema::{users, public_keys, projects};
use iron::typemap::Key;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[has_many(public_keys)]
#[has_many(projects)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email_address: String,
    pub bcrypt_hash: String,
    pub created_at: NaiveDateTime,
    pub screen_name: String,
    pub is_admin: bool,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email_address: &'a str,
    pub bcrypt_hash: &'a str,
    pub screen_name: Option<&'a str>,
    pub is_admin: Option<bool>,
}

impl Key for User {
    type Value = User;
}
