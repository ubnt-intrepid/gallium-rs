pub mod v1;

use std::fmt;
use std::error;

#[derive(Debug)]
pub struct ApiError(&'static str);

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl error::Error for ApiError {
    fn description(&self) -> &str {
        self.0
    }
}
