use bcrypt;
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

    pub fn open_repository(
        &self,
        user: &str,
        project: &str,
    ) -> Result<(User, Project, Repository), AppError> {
        let conn = self.get_db_conn()?;
        let (user, project) =
            users::table
                .inner_join(projects::table)
                .filter(users::dsl::name.eq(&user))
                .filter(projects::dsl::name.eq(project))
                .get_result::<(User, Project)>(&*conn)
                .optional()?
                .ok_or_else(|| AppError::Other("The repository has not created yet"))?;

        let repo_path = self.generate_repository_path(&user.name, &project.name);
        if !repo_path.is_dir() {
            return Err(AppError::Other("Not found"));
        }
        let repo = Repository::open(repo_path)?;
        Ok((user, project, repo))
    }

    pub fn open_repository_from_id(
        &self,
        id: i32,
    ) -> Result<Option<(User, Project, Repository)>, AppError> {
        let conn = self.get_db_conn()?;
        let result = users::table
            .inner_join(projects::table)
            .filter(projects::dsl::id.eq(id))
            .get_result::<(User, Project)>(&*conn)
            .optional()?;
        match result {
            Some((user, project)) => {
                let repo_path = self.generate_repository_path(&user.name, &project.name);
                if !repo_path.is_dir() {
                    return Err(AppError::Other(""));
                }
                let repo = Repository::open(repo_path)?;
                Ok(Some((user, project, repo)))
            }
            None => Ok(None),
        }
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<Option<User>, AppError> {
        let conn = self.get_db_conn()?;
        let user = users::table
            .filter(users::dsl::name.eq(username))
            .get_result::<User>(&*conn)
            .optional()?
            .and_then(|user| {
                let verified = bcrypt::verify(password, &user.bcrypt_hash).unwrap_or(false);
                if verified { Some(user) } else { None }
            });
        Ok(user)
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
