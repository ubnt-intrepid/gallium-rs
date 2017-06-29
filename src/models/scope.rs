use std::str::FromStr;
use error::AppError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Read,
    Write,
}

impl FromStr for Scope {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Scope::Read),
            "write" => Ok(Scope::Write),
            _ => Err(AppError::from("Failed to parse scope value")),
        }
    }
}
