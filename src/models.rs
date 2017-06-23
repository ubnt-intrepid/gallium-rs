use super::schema::{users, public_keys, projects};
use chrono::NaiveDateTime;

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

#[derive(Serialize)]
pub struct EncodablePublicKey {
    id: i32,
    created_at: String,
    title: String,
    key: String,
}

impl From<PublicKey> for EncodablePublicKey {
    fn from(val: PublicKey) -> Self {
        EncodablePublicKey {
            id: val.id,
            created_at: val.created_at.format("%c").to_string(),
            title: val.title,
            key: val.key,
        }
    }
}



#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[has_many(public_keys)]
#[has_many(projects)]
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

#[derive(Serialize)]
pub struct EncodableUser {
    id: i32,
    username: String,
    email_address: String,
    created_at: String,
}

impl From<User> for EncodableUser {
    fn from(val: User) -> Self {
        EncodableUser {
            id: val.id,
            username: val.username,
            email_address: val.email_address,
            created_at: val.created_at.format("%c").to_string(),
        }
    }
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
    pub description: &'a str,
}

#[derive(Debug, Serialize)]
pub struct EncodableProject {
    pub id: i32,
    pub created_at: String,
    pub user_id: i32,
    pub name: String,
    pub description: String,
}

impl From<Project> for EncodableProject {
    fn from(val: Project) -> Self {
        EncodableProject {
            id: val.id,
            created_at: val.created_at.format("%c").to_string(),
            user_id: val.user_id,
            name: val.name,
            description: val.description,
        }
    }
}
