mod keys;
mod users;
mod projects;

use iron::prelude::*;
use router::Router;
use iron_json_response::JsonResponseMiddleware;

pub fn create_api_handler() -> Chain {
    let mut router = Router::new();
    router.get("/keys", keys::handle_get_ssh_keys, "get_ssh_keys");
    router.post("/keys", keys::handle_add_ssh_key, "add_ssh_key");

    router.post("/users", users::create_user, "create_user");

    router.get("/projects", projects::get_projecs, "get_projects");
    router.post("/projects", projects::create_project, "create_project");

    let mut chain = Chain::new(router);
    chain.link_after(JsonResponseMiddleware::new());

    chain
}
