use chrono::NaiveDateTime;
use schema::{users, projects};
use super::users::User;
use db::DB;
use error::{AppResult, AppError};
use super::repository;

use diesel::prelude::*;
use diesel::insert;


#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct Project {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
}

impl Project {
    pub fn create(db: &DB, user: &str, name: &str, description: Option<&str>) -> AppResult<Self> {
        if repository::open_repository(db, user, name).is_ok() {
            return Err(AppError::from("The repository has already created."));
        }
        repository::Repository::create(format!("{}/{}", user, name))?;

        let conn = db.get_db_conn()?;

        let user_id: i32 = users::table
            .filter(users::dsl::name.eq(&user))
            .select(users::dsl::id)
            .get_result(&*conn)?;

        let new_project = NewProject {
            name,
            user_id,
            description,
        };

        insert(&new_project)
            .into(projects::table)
            .get_result::<Project>(&*conn)
            .map_err(Into::into)
    }
}

#[derive(Insertable)]
#[table_name = "projects"]
pub struct NewProject<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub description: Option<&'a str>,
}
