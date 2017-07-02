extern crate gallium;

use gallium::Config;
use std::env;

fn main() {
    let config = Config::load().unwrap();
    env::set_current_dir(&config.repository_root).unwrap();

    gallium::server::start(config).unwrap();
}
