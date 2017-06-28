use bcrypt;
use std::sync::Arc;
use iron::{Request, IronResult, BeforeMiddleware};
use config::Config;
use diesel::prelude::*;
use models::{User, Project, OAuthApp};
use schema::{users, projects, oauth_apps};
use models::Repository;
use error::AppResult;
use db::DB;

pub fn create_new_repository(config: &Config, user: &str, project: &str) -> AppResult<()> {
    let repo_path = config.repository_path(user, project);
    Repository::create(&repo_path)?;
    Ok(())
}

pub fn open_repository(
    db: &DB,
    config: &Config,
    user: &str,
    project: &str,
) -> AppResult<Option<(User, Project, Repository)>> {
    let conn = db.get_db_conn()?;
    let result = users::table
        .inner_join(projects::table)
        .filter(users::dsl::name.eq(&user))
        .filter(projects::dsl::name.eq(project))
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

pub fn authenticate(db: &DB, username: &str, password: &str) -> AppResult<Option<User>> {
    let conn = db.get_db_conn()?;
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

pub fn authenticate_app(db: &DB, client_id: &str, client_secret: &str) -> AppResult<Option<OAuthApp>> {
    let conn = db.get_db_conn()?;
    let app = oauth_apps::table
        .filter(oauth_apps::dsl::client_id.eq(client_id))
        .get_result::<OAuthApp>(&*conn)
        .optional()?
        .and_then(|app| if app.client_secret == client_secret {
            Some(app)
        } else {
            None
        });
    Ok(app)
}


pub struct AppMiddleware {
    config: Arc<Config>,
    db: DB,
}

impl AppMiddleware {
    pub fn new(config: Config) -> AppResult<Self> {
        let db = DB::new(&config.database_url)?;
        Ok(AppMiddleware {
            config: Arc::new(config),
            db,
        })
    }
}

impl BeforeMiddleware for AppMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<DB>(self.db.clone());
        req.extensions.insert::<Config>(self.config.clone());
        Ok(())
    }
}
