use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub repository_root: PathBuf,
}

impl Config {
    pub fn from_env_vars() -> Config {
        Config {
            database_url: env::var("DATABASE_URL").unwrap(),
            repository_root: env::var("REPOSITORY_ROOT").ok().map(Into::into).unwrap(),
        }
    }
}
