use std::{env, fs, path};
use serde_json;
use error::AppResult;


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
}
