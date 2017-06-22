extern crate gallium;
extern crate dotenv;
extern crate iron;

use gallium::config::Config;
use gallium::routes;
use dotenv::dotenv;
use iron::prelude::*;

fn main() {
    dotenv().ok();
    let config = Config::from_env_vars();
    let router = routes::create_handler(config);

    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
