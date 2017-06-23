use std::{env, error, fmt, fs, io, path};
use serde_json;


#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    SerdeJson(serde_json::Error),
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "config error")
    }
}
impl error::Error for ConfigError {
    fn description(&self) -> &str {
        "config error"
    }
}
impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> ConfigError {
        ConfigError::Io(err)
    }
}
impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> ConfigError {
        ConfigError::SerdeJson(err)
    }
}


#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub repository_root: path::PathBuf,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
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
