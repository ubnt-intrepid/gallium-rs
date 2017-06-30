use chrono::NaiveDateTime;
use super::users::User;
use schema::access_tokens;
use db::DB;
use crypto;
use error::AppResult;
use diesel::prelude::*;
use diesel::insert;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct AccessToken {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub hash: String,
}

impl AccessToken {
    pub fn create(db: &DB, user_id: i32) -> AppResult<AccessToken> {
        let token_hash = crypto::generate_sha1_random();
        let new_token = NewAccessToken {
            user_id,
            hash: &token_hash,
        };
        let conn = db.get_db_conn()?;
        insert(&new_token)
            .into(access_tokens::table)
            .get_result(&*conn)
            .map_err(Into::into)
    }
}

#[derive(Insertable)]
#[table_name = "access_tokens"]
pub struct NewAccessToken<'a> {
    pub user_id: i32,
    pub hash: &'a str,
}
