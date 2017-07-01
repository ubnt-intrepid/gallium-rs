mod error;
mod ssh_keys;
mod projects;
mod repository;
mod users;

use iron::prelude::*;
use iron::method::Method;
use router::Router;

pub fn create_api_handler() -> Chain {
    let mut router = Router::new();
    router.register(projects::GetProjects);
    router.register(projects::GetProject);
    router.register(projects::CreateProject);
    router.register(projects::DeleteProject);
    router.register(repository::ShowTree);
    router.register(ssh_keys::GetKeys);
    router.register(ssh_keys::GetKey);
    router.register(ssh_keys::AddKey);
    router.register(ssh_keys::DeleteKey);
    router.register(users::GetUsers);
    router.register(users::GetUser);
    router.register(users::CreateUser);
    Chain::new(router)
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
