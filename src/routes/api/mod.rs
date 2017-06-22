mod keys;
mod users;

use std::error;
use std::fmt;
use iron::prelude::*;
use router::Router;
use iron_json_response::JsonResponseMiddleware;
use app::{App, AppMiddleware};
use config::Config;


#[derive(Debug)]
pub struct ApiError;

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "API error")
    }
}

impl error::Error for ApiError {
    fn description(&self) -> &str {
        "API error"
    }
}


pub fn create_api_handler(config: Config) -> Chain {
    let app = App::new(config).unwrap();

    let mut router = Router::new();
    router.get("/user/keys", keys::handle_get_ssh_keys, "get_ssh_keys");
    router.post("/user/keys", keys::handle_add_ssh_key, "add_ssh_key");
    router.post("/users", users::create_user, "create_user");

    let mut chain = Chain::new(router);
    chain.link_before(AppMiddleware::new(app));
    chain.link_after(JsonResponseMiddleware::new());

    chain
}
