extern crate gallium;
extern crate iron;

use iron::prelude::*;
use gallium::config::Config;
use std::env;

fn main() {
    let config = Config::load().unwrap();
    env::set_current_dir(&config.repository_root).unwrap();

    let router = gallium::routes::create_handler(config).unwrap();
    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
