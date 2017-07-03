use bcrypt;
use chrono::NaiveDateTime;
use diesel::insert;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use iron::typemap::Key;

use error::AppResult;
use schema::{users, ssh_keys, projects};


#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[has_many(ssh_keys)]
#[has_many(projects)]
pub struct User {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub name: String,
    pub screen_name: Option<String>,
    pub bcrypt_hash: String,
}

#[derive(Insertable)]
#[table_name = "users"]
struct NewUser<'a> {
    name: &'a str,
    screen_name: Option<&'a str>,
    bcrypt_hash: &'a str,
}


impl User {
    pub fn create(conn: &PgConnection, name: &str, password: &str, screen_name: Option<&str>) -> AppResult<Self> {
        let bcrypt_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let new_user = NewUser {
            name: name,
            bcrypt_hash: &bcrypt_hash,
            screen_name: screen_name,
        };

        insert(&new_user)
            .into(users::table)
            .get_result::<User>(&*conn)
            .map_err(Into::into)
    }

    pub fn load_users(conn: &PgConnection) -> AppResult<Vec<Self>> {
        users::table.load::<User>(&*conn).map_err(Into::into)
    }

    pub fn find_by_id(conn: &PgConnection, id: i32) -> AppResult<Option<Self>> {
        users::table
            .filter(users::dsl::id.eq(id))
            .get_result::<User>(&*conn)
            .optional()
            .map_err(Into::into)
    }

    pub fn authenticate(conn: &PgConnection, username: &str, password: &str) -> AppResult<Option<Self>> {
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
