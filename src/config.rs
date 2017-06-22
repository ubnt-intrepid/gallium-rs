use std::env;

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
}

impl Config {
    pub fn from_env_vars() -> Config {
        Config { database_url: env::var("DATABASE_URL").unwrap() }
    }
}
