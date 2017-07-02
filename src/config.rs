use std::{env, fs, path};
use std::sync::Arc;
use serde_json;
use error::AppResult;
use iron::{Request, IronResult, BeforeMiddleware};
use iron::typemap::Key;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub repository_root: path::PathBuf,
    pub jwt_secret: String,
}

impl Config {
    pub fn load() -> AppResult<Self> {
        let conf_path = env::current_exe()?
            .parent()
            .unwrap()
            .join("../conf/config.json")
            .canonicalize()?;
        let mut f = fs::OpenOptions::new().read(true).open(conf_path)?;
        let config = serde_json::from_reader(&mut f)?;
        Ok(config)
    }

    pub fn repository_path(&self, user: &str, project: &str) -> path::PathBuf {
        self.repository_root.join(user).join(project)
    }
}

impl Key for Config {
    type Value = Arc<Config>;
}


pub struct ConfigMiddleware(Arc<Config>);

impl ConfigMiddleware {
    pub fn new(config: Config) -> Self {
        ConfigMiddleware(Arc::new(config))
    }
}

impl BeforeMiddleware for ConfigMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<Config>(self.0.clone());
        Ok(())
    }
}
