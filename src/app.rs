use bcrypt;
use chrono::UTC;
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
use git::Repository;
use error::AppResult;
use jsonwebtoken;
use uuid::Uuid;
use std::time::Duration;

type DbPool = Pool<ConnectionManager<PgConnection>>;
type PooledDbConnection = PooledConnection<ConnectionManager<PgConnection>>;


#[derive(Debug, Deserialize)]
pub struct JWTClaims {
    pub user_id: i32,
    pub username: String,
    pub scope: Vec<String>,
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
    ) -> AppResult<(User, Project, Repository)> {
        let conn = self.get_db_conn()?;
        let (user, project) = users::table
            .inner_join(projects::table)
            .filter(users::dsl::name.eq(&user))
            .filter(projects::dsl::name.eq(project))
            .get_result::<(User, Project)>(&*conn)
            .optional()?
            .ok_or_else(|| "The repository has not created yet")?;

        let repo_path = self.generate_repository_path(&user.name, &project.name);
        if !repo_path.is_dir() {
            return Err("Not found".into());
        }
        let repo = Repository::open(repo_path)?;
        Ok((user, project, repo))
    }

    pub fn open_repository_from_id(
        &self,
        id: i32,
    ) -> AppResult<Option<(User, Project, Repository)>> {
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
                    return Err("".into());
                }
                let repo = Repository::open(repo_path)?;
                Ok(Some((user, project, repo)))
            }
            None => Ok(None),
        }
    }

    pub fn authenticate(&self, username: &str, password: &str) -> AppResult<Option<User>> {
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

    pub fn generate_jwt(
        &self,
        user: &User,
        scope: Option<&[&str]>,
        lifetime: Duration,
    ) -> AppResult<String> {
        let iss = "http://localhost:3000/";
        let aud = vec!["http://localhost:3000/"];

        let jti = Uuid::new_v4();
        let iat = UTC::now();
        let claims = json!({
            "jti": jti.to_string(),
            "iss": iss,
            "aud": aud,
            "sub": "access_token",
            "iat": iat.timestamp(),
            "nbf": iat.timestamp(),
            "exp": iat.timestamp() + lifetime.as_secs() as i64,
            "user_id": user.id,
            "username": user.name,
            "scope": scope,
        });
        jsonwebtoken::encode(
            &Default::default(),
            &claims,
            self.config.jwt_secret.as_bytes(),
        ).map_err(Into::into)
    }

    pub fn validate_jwt(&self, token: &str) -> AppResult<JWTClaims> {
        jsonwebtoken::decode(
            token,
            self.config.jwt_secret.as_bytes(),
            &Default::default(),
        ).map_err(Into::into)
            .map(|token_data| token_data.claims)
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
