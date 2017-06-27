use super::schema::{users, public_keys, projects};
use chrono::NaiveDateTime;
use iron::typemap;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct PublicKey {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub key: String,
    pub user_id: i32,
    pub title: String,
}

#[derive(Insertable)]
#[table_name = "public_keys"]
pub struct NewPublicKey<'a> {
    pub key: &'a str,
    pub user_id: i32,
    pub title: &'a str,
}


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

impl typemap::Key for User {
    type Value = User;
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


#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct Project {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Insertable)]
#[table_name = "projects"]
pub struct NewProject<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub description: Option<&'a str>,
}
