extern crate gallium;
extern crate iron;

use iron::prelude::*;

fn main() {
    let router = gallium::routes::create_handler().unwrap();
    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
