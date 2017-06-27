use chrono::NaiveDateTime;
use super::users::User;
use schema::public_keys;

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
