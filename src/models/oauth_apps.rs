use schema::oauth_apps;
use chrono::NaiveDateTime;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[table_name = "oauth_apps"]
pub struct OAuthApp {
    pub id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub client_id: String,
    pub redirect_uri: String,
    pub client_secret: String,
}

#[derive(Insertable)]
#[table_name = "oauth_apps"]
pub struct NewOAuthApp<'a> {
    pub name: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: Option<&'a str>,
    pub client_secret: &'a str,
}
