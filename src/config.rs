use std::env::{self, VarError};
use std::io;
use std::path::PathBuf;
use serde_json;
use std::fs::OpenOptions;
use std::fmt;
use std::error;

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    EnvVar(VarError),
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
impl From<VarError> for ConfigError {
    fn from(err: VarError) -> ConfigError {
        ConfigError::EnvVar(err)
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
    pub repository_root: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let conf_path = env::current_exe()?
            .parent()
            .unwrap()
            .join("../conf/config.json")
            .canonicalize()?;
        let mut f = OpenOptions::new().read(true).open(conf_path)?;
        let mut buf = Vec::new();
        io::copy(&mut f, &mut buf)?;
        let config = serde_json::from_slice(&buf)?;
        Ok(config)
    }

    #[deprecated]
    pub fn from_env_vars() -> Result<Self, ConfigError> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")?,
            repository_root: env::var("REPOSITORY_ROOT").map(Into::into)?,
        })
    }
}
