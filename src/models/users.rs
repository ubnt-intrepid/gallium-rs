use bcrypt;
use chrono::NaiveDateTime;
use diesel::insert;
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
struct NewUser<'a> {
    name: &'a str,
    email_address: &'a str,
    bcrypt_hash: &'a str,
    screen_name: Option<&'a str>,
    is_admin: Option<bool>,
}


impl User {
    pub fn create(
        db: &DB,
        name: &str,
        password: &str,
        email_address: &str,
        screen_name: Option<&str>,
        is_admin: Option<bool>,
    ) -> AppResult<Self> {
        let bcrypt_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let new_user = NewUser {
            name: name,
            email_address: email_address,
            bcrypt_hash: &bcrypt_hash,
            screen_name: screen_name,
            is_admin: is_admin,
        };

        let conn = db.get_db_conn()?;
        insert(&new_user)
            .into(users::table)
            .get_result::<User>(&*conn)
            .map_err(Into::into)
    }

    pub fn load_users(db: &DB) -> AppResult<Vec<Self>> {
        let conn = db.get_db_conn()?;
        users::table.load::<User>(&*conn).map_err(Into::into)
    }

    pub fn find_by_id(db: &DB, id: i32) -> AppResult<Option<Self>> {
        let conn = db.get_db_conn()?;
        users::table
            .filter(users::dsl::id.eq(id))
            .get_result::<User>(&*conn)
            .optional()
            .map_err(Into::into)
    }

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
