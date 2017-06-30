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

#[derive(Clone, Debug, Insertable, Deserialize)]
#[table_name = "ssh_keys"]
pub struct NewSshKey {
    pub key: String,
    pub user_id: i32,
    pub description: Option<String>,
}
