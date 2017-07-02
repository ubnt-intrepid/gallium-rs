use chrono::NaiveDateTime;
use schema::{users, projects};
use super::users::User;
use super::repository::Repository;
use db::DB;
use error::AppResult;

use diesel::prelude::*;
use diesel::insert;
use std::path::Path;


#[derive(Debug)]
pub enum ProjectID {
    Number(i32),
    Path(String, String),
}

impl From<i32> for ProjectID {
    fn from(id: i32) -> Self {
        ProjectID::Number(id)
    }
}

impl<A, B> From<(A, B)> for ProjectID
where
    A: AsRef<str>,
    B: AsRef<str>,
{
    fn from(path: (A, B)) -> Self {
        ProjectID::Path(path.0.as_ref().to_owned(), path.1.as_ref().to_owned())
    }
}


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

    pub fn find_by_id<I: Into<ProjectID>>(db: &DB, id: I) -> AppResult<Option<Self>> {
        let conn = db.get_db_conn()?;
        match id.into() {
            ProjectID::Number(id) => {
                projects::table
                    .filter(projects::dsl::id.eq(id))
                    .get_result::<Project>(&*conn)
                    .optional()
                    .map_err(Into::into)
            }
            ProjectID::Path(ref user, ref project) => {
                users::table
                    .inner_join(projects::table)
                    .filter(users::dsl::name.eq(user.as_str()))
                    .filter(projects::dsl::name.eq(project.as_str()))
                    .get_result::<(User, Project)>(&*conn)
                    .map(|(_, project)| project)
                    .optional()
                    .map_err(Into::into)
            }
        }
    }

    pub fn open_repository(&self, db: &DB) -> AppResult<Repository> {
        let conn = db.get_db_conn()?;
        let user = users::table
            .filter(users::dsl::id.eq(self.user_id))
            .get_result::<User>(&*conn)?;
        let repo_path = Path::new(&format!("{}/{}", user.name, self.name)).to_path_buf();
        Repository::open(repo_path)
    }
}
