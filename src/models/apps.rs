use diesel::prelude::*;
use chrono::NaiveDateTime;

use error::AppResult;
use db::DB;
use models::User;
use schema::apps;


#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[table_name = "apps"]
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
#[table_name = "apps"]
pub struct NewOAuthApp<'a> {
    pub name: &'a str,
    pub user_id: i32,
    pub client_id: &'a str,
    pub redirect_uri: Option<&'a str>,
    pub client_secret: &'a str,
}


impl OAuthApp {
    pub fn authenticate(db: &DB, client_id: &str, client_secret: &str) -> AppResult<Option<Self>> {
        let conn = db.get_db_conn()?;
        let app = apps::table
            .filter(apps::dsl::client_id.eq(client_id))
            .get_result::<OAuthApp>(&*conn)
            .optional()?
            .and_then(|app| if app.client_secret == client_secret {
                Some(app)
            } else {
                None
            });
        Ok(app)
    }
}
