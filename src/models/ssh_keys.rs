use chrono::NaiveDateTime;
use super::users::User;
use schema::ssh_keys;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct SshKey {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub key: String,
    pub user_id: i32,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[table_name = "ssh_keys"]
pub struct NewSshKey<'a> {
    pub key: &'a str,
    pub user_id: i32,
    pub description: Option<&'a str>,
}
