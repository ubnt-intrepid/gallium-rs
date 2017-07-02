use chrono::NaiveDateTime;
use schema::{users, projects};
use super::users::User;
use super::repository::Repository;
use db::DB;
use config::Config;
use error::{AppResult, AppError};

use diesel::prelude::*;
use diesel::insert;
use std::path::Path;


#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct Project {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[table_name = "projects"]
pub struct NewProject<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub description: Option<&'a str>,
}

impl Project {
    pub fn create(db: &DB, user: &str, name: &str, description: Option<&str>) -> AppResult<Self> {
        if open_repository(db, user, name).is_ok() {
            return Err(AppError::from("The repository has already created."));
        }
        Repository::create(format!("{}/{}", user, name))?;

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

pub fn open_repository(db: &DB, user: &str, project: &str) -> AppResult<Option<(User, Project, Repository)>> {
    let conn = db.get_db_conn()?;
    let result = users::table
        .inner_join(projects::table)
        .filter(users::dsl::name.eq(&user))
        .filter(projects::dsl::name.eq(project))
        .get_result::<(User, Project)>(&*conn)
        .optional()?;
    match result {
        Some((user, project)) => {
            let repo_path = Path::new(&format!("{}/{}", user.name, project.name)).to_path_buf();
            if !repo_path.is_dir() {
                return Err("".into());
            }
            let repo = Repository::open(repo_path)?;
            Ok(Some((user, project, repo)))
        }
        None => Ok(None),
    }
}

pub fn open_repository_from_id(db: &DB, config: &Config, id: i32) -> AppResult<Option<(User, Project, Repository)>> {
    let conn = db.get_db_conn()?;
    let result = users::table
        .inner_join(projects::table)
        .filter(projects::dsl::id.eq(id))
        .get_result::<(User, Project)>(&*conn)
        .optional()?;
    match result {
        Some((user, project)) => {
            let repo_path = config.repository_path(&user.name, &project.name);
            if !repo_path.is_dir() {
                return Err("".into());
            }
            let repo = Repository::open(repo_path)?;
            Ok(Some((user, project, repo)))
        }
        None => Ok(None),
    }
}
