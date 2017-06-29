use chrono::NaiveDateTime;
use super::users::User;
use super::apps::OAuthApp;
use schema::access_tokens;
use db::DB;
use crypto;
use error::AppResult;
use diesel::prelude::*;
use diesel::insert;
use models::Scope;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
#[belongs_to(OAuthApp, foreign_key = "oauth_app_id")]
pub struct AccessToken {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub oauth_app_id: i32,
    pub hash: String,
}

impl AccessToken {
    pub fn create<I>(db: &DB, user_id: i32, oauth_app_id: i32, _scope: Option<I>) -> AppResult<AccessToken>
    where
        I: IntoIterator<Item = Scope>,
    {
        let token_hash = crypto::generate_sha1_random();
        let new_token = NewAccessToken {
            user_id,
            oauth_app_id,
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
    pub oauth_app_id: i32,
    pub hash: &'a str,
}
