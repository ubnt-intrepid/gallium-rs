use std::error;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use iron::{Request, IronResult, BeforeMiddleware};
use iron::typemap;
use config::Config;
use r2d2::{Pool, PooledConnection, InitializationError, GetTimeout};
use r2d2_diesel::ConnectionManager;
use diesel::pg::PgConnection;

type DbPool = Pool<ConnectionManager<PgConnection>>;
type PooledDbConnection = PooledConnection<ConnectionManager<PgConnection>>;


#[derive(Debug)]
pub struct AppError(&'static str);
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl error::Error for AppError {
    fn description(&self) -> &str {
        self.0
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
        // TODO: get repository path from DB
        let repo_path = self.generate_repository_path(user, project);
        if !repo_path.is_dir() {
            return Err(AppError("Not found"));
        }
        Ok(repo_path)
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
