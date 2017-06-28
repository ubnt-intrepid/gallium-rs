use bcrypt;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use iron::typemap::Key;

use db::DB;
use error::AppResult;
use schema::{users, public_keys, projects};


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


impl User {
    pub fn authenticate(db: &DB, username: &str, password: &str) -> AppResult<Option<Self>> {
        let conn = db.get_db_conn()?;
        let user = users::table
            .filter(users::dsl::name.eq(username))
            .get_result::<User>(&*conn)
            .optional()?
            .and_then(|user| {
                let verified = bcrypt::verify(password, &user.bcrypt_hash).unwrap_or(false);
                if verified { Some(user) } else { None }
            });
        Ok(user)
    }
}

impl Key for User {
    type Value = User;
}
