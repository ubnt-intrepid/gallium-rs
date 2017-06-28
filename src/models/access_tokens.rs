use chrono::NaiveDateTime;
use super::users::User;
use super::oauth_apps::OAuthApp;
use schema::access_tokens;

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

#[derive(Insertable)]
#[table_name = "access_tokens"]
pub struct NewAccessToken<'a> {
    pub user_id: i32,
    pub oauth_app_id: i32,
    pub hash: &'a str,
}
