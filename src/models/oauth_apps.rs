use schema::oauth_apps;
use chrono::NaiveDateTime;
use super::users::User;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[table_name = "oauth_apps"]
#[belongs_to(User)]
pub struct OAuthApp {
    pub id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub client_id: String,
    pub redirect_uri: String,
    pub client_secret: String,
    pub user_id: i32,
}

#[derive(Insertable)]
#[table_name = "oauth_apps"]
pub struct NewOAuthApp<'a> {
    pub name: &'a str,
    pub user_id: i32,
    pub client_id: &'a str,
    pub redirect_uri: Option<&'a str>,
    pub client_secret: &'a str,
}
