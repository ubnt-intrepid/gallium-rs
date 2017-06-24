use std::error;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use iron::{Request, IronResult, BeforeMiddleware};
use iron::typemap;
use config::Config;
use r2d2::{Pool, PooledConnection, InitializationError, GetTimeout};
use r2d2_diesel::ConnectionManager;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use models::{User, Project};
use schema::{users, projects};
use git2;
use git::Repository;

type DbPool = Pool<ConnectionManager<PgConnection>>;
type PooledDbConnection = PooledConnection<ConnectionManager<PgConnection>>;


#[derive(Debug)]
pub enum AppError {
    Diesel(::diesel::result::Error),
    R2D2(GetTimeout),
    Git2(git2::Error),
    Other(&'static str),
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppError::Diesel(ref err) => write!(f, "{}", err),
            AppError::R2D2(ref err) => write!(f, "{}", err),
            AppError::Git2(ref err) => write!(f, "{}", err),
            AppError::Other(ref err) => write!(f, "{}", err),
        }
    }
}
impl error::Error for AppError {
    fn description(&self) -> &str {
        match *self {
            AppError::Diesel(ref err) => err.description(),
            AppError::R2D2(ref err) => err.description(),
            AppError::Git2(ref err) => err.description(),
            AppError::Other(ref err) => err,
        }
    }
}
impl From<GetTimeout> for AppError {
    fn from(err: GetTimeout) -> Self {
        AppError::R2D2(err)
    }
}
impl From<::diesel::result::Error> for AppError {
    fn from(err: ::diesel::result::Error) -> Self {
        AppError::Diesel(err)
    }
}
impl From<git2::Error> for AppError {
    fn from(err: git2::Error) -> Self {
        AppError::Git2(err)
    }
}


pub struct App {
    config: Config,
    db_pool: DbPool,
}

impl App {
    pub fn new(config: Config) -> Result<Self, InitializationError> {
        let manager = ConnectionManager::new(config.database_url.as_str());
        let db_pool = Pool::new(Default::default(), manager)?;
        Ok(App { config, db_pool })
    }

    pub fn get_db_conn(&self) -> Result<PooledDbConnection, GetTimeout> {
        self.db_pool.get()
    }

    pub fn generate_repository_path(&self, user: &str, project: &str) -> PathBuf {
        self.config.repository_root.join(user).join(project)
    }

    pub fn resolve_repository_path(&self, user: &str, project: &str) -> Result<PathBuf, AppError> {
        if !project.ends_with(".git") {
            return Err(AppError::Other("Not found"));
        }
        let project = project.trim_right_matches(".git");

        // get repository info from DB
        use diesel::prelude::*;
        use models::Project;
        use schema::{users, projects};
        let conn = self.get_db_conn()?;
        let user_id: i32 = users::table
            .filter(users::dsl::username.eq(&user))
            .select(users::dsl::id)
            .get_result(&*conn)?;
        let projects = projects::table
            .filter(projects::dsl::user_id.eq(user_id))
            .filter(projects::dsl::name.eq(project))
            .load::<Project>(&*conn)?;
        if projects.len() == 0 {
            return Err(AppError::Other("The repository has not created yet"));
        }

        let repo_path = self.generate_repository_path(user, project);
        if !repo_path.is_dir() {
            return Err(AppError::Other("Not found"));
        }
        Ok(repo_path)
    }

    pub fn get_repository(&self, project_pair: (User, Project)) -> Result<Repository, AppError> {
        let (user, project) = project_pair;
        let username = user.username;
        let proj_name = format!("{}.git", project.name);
        let repo_path = self.resolve_repository_path(&username, &proj_name)?;

        Repository::open(repo_path).map_err(Into::into)
    }

    pub fn get_project_from_id(&self, id: i32) -> Result<(User, Project), AppError> {
        let conn = self.get_db_conn()?;
        users::table
            .inner_join(projects::table)
            .filter(projects::dsl::id.eq(id))
            .get_result::<(User, Project)>(&*conn)
            .map_err(Into::into)
    }
}


impl typemap::Key for App {
    type Value = Arc<App>;
}

pub struct AppMiddleware {
    app: Arc<App>,
}

impl AppMiddleware {
    pub fn new(app: App) -> Self {
        AppMiddleware { app: Arc::new(app) }
    }
}

impl BeforeMiddleware for AppMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<App>(self.app.clone());
        Ok(())
    }
}
