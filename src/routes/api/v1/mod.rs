mod error;
mod ssh_keys;
mod projects;
mod repository;
mod users;

use iron::prelude::*;
use mount::Mount;

pub fn create_api_handler() -> Chain {
    let mut mount = Mount::new();
    mount.mount("/ssh_keys", ssh_keys::create_routes());
    mount.mount("/projects", projects::create_routes());
    mount.mount("/users", users::create_routes());

    Chain::new(mount)
}
