use chrono::NaiveDateTime;
use schema::{users, projects};
use super::users::User;
use super::repository::Repository;
use db::DB;
use error::AppResult;

use diesel::prelude::*;
use diesel::pg::PgConnection;


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

impl Project {
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

    pub fn open_repository(&self, conn: &PgConnection) -> AppResult<Repository> {
        let user = users::table
            .filter(users::dsl::id.eq(self.user_id))
            .get_result::<User>(conn)?;
        Repository::open(format!("{}/{}", user.name, self.name))
    }

    pub fn init_repository(&self, conn: &PgConnection) -> AppResult<Repository> {
        let user = users::table
            .filter(users::dsl::id.eq(self.user_id))
            .get_result::<User>(conn)?;
        let repo = Repository::init(format!("{}/{}", user.name, self.name))?;
        Ok(repo)
    }
}


#[derive(Clone, Deserialize)]
pub struct NewProject {
    pub user: String,
    pub name: String,
    pub description: Option<String>,
}

impl NewProject {
    pub fn insert(&self, db: &DB) -> AppResult<Project> {
        use diesel::types::{Int4, Timestamp, Text, Nullable};
        use diesel::expression::dsl::sql;

        let query = sql::<(Int4, Timestamp, Int4, Text, Nullable<Text>)>(&format!(
            "INSERT INTO projects (user_id, name, description)
             SELECT id, {}, {} FROM users
             WHERE users.name = {} LIMIT 1
             RETURNING *",
            escape_str(&self.name),
            self.description.as_ref().map(|s| escape_str(&s)).unwrap_or("NULL".to_owned()),
            escape_str(&self.user),
        ));

        let conn = db.get_db_conn()?;
        conn.transaction(|| -> AppResult<Project> {
            let project: Project = query.get_result(&*conn)?;
            project.init_repository(&*conn)?;
            Ok(project)
        })
    }
}



fn escape_str(s: &str) -> String {
    format!("'{}'", s.replace("'", "''"))
}
