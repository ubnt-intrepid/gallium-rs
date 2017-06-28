extern crate gallium;
extern crate iron;

use gallium::app::AppMiddleware;
use gallium::config::Config;
use gallium::routes;
use iron::prelude::*;

fn main() {
    let config = Config::load().unwrap();
    let app = AppMiddleware::new(config).unwrap();

    let router = routes::create_handler(app);
    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
