extern crate gallium;
extern crate iron;

use gallium::routes;
use iron::prelude::*;

fn main() {
    let router = routes::create_handler();
    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
