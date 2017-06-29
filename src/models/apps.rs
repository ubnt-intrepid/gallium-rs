use diesel::insert;
use diesel::prelude::*;
use chrono::NaiveDateTime;

use crypto;
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
    pub fn create(db: &DB, name: &str, user_id: i32, redirect_uri: Option<&str>) -> AppResult<Self> {
        let client_id = crypto::generate_sha1_hash();
        let client_secret = crypto::generate_sha1_random();

        let new_app = NewOAuthApp {
            name,
            user_id,
            redirect_uri: redirect_uri,
            client_id: &client_id,
            client_secret: &client_secret,
        };
        let conn = db.get_db_conn()?;
        insert(&new_app)
            .into(apps::table)
            .get_result::<OAuthApp>(&*conn)
            .map_err(Into::into)
    }

    pub fn load_apps(db: &DB) -> AppResult<Vec<Self>> {
        let conn = db.get_db_conn()?;
        apps::table.load(&*conn).map_err(Into::into)
    }

    pub fn find_by_id(db: &DB, id: i32) -> AppResult<Option<Self>> {
        let conn = db.get_db_conn()?;
        apps::table
            .filter(apps::dsl::id.eq(id))
            .get_result(&*conn)
            .optional()
            .map_err(Into::into)
    }

    pub fn find_by_client_id(db: &DB, client_id: &str) -> AppResult<Option<Self>> {
        let conn = db.get_db_conn()?;
        apps::table
            .filter(apps::dsl::client_id.eq(client_id))
            .get_result::<OAuthApp>(&*conn)
            .optional()
            .map_err(Into::into)
    }

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
