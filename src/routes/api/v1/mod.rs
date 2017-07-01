mod error;
mod ssh_keys;
mod projects;
mod repository;
mod users;

use iron::prelude::*;
use iron::method::Method;
use mount::Mount;
use router::Router;

pub fn create_api_handler() -> Chain {
    let mut mount = Mount::new();
    mount.mount("/ssh_keys", ssh_keys::create_routes());
    mount.mount("/projects", projects::create_routes());
    mount.mount("/users", users::create_routes());

    Chain::new(mount)
}



trait Route {
    fn route_path() -> &'static str;
    fn route_method() -> Method;
    fn route_id() -> &'static str;
}

trait RegisterRoute {
    fn register<R: Route + Into<Chain>>(&mut self, route: R) -> &mut Self;
}

impl RegisterRoute for Router {
    fn register<R: Route + Into<Chain>>(&mut self, route: R) -> &mut Self {
        self.route(
            R::route_method(),
            R::route_path(),
            route.into(),
            R::route_id(),
        )
    }
}
