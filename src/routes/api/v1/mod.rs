mod error;
mod ssh_keys;
mod projects;
mod repository;
mod users;

use iron::prelude::*;
use router::Router;
use iron_json_response::JsonResponseMiddleware;

pub fn create_api_handler() -> Chain {
    let mut router = Router::new();

    router.get("/ssh_keys", ssh_keys::get_keys, "keys/get_keys");
    router.get("/ssh_keys/:id", ssh_keys::get_key, "keys/get_key");
    router.post("/ssh_keys", ssh_keys::add_key, "keys/add_key");
    router.delete("/ssh_keys/:id", ssh_keys::delete_key, "keys/delete_key");

    router.get("/projects", projects::get_projecs, "get_projects");
    router.get("/projects/:id", projects::get_project, "get_project");
    router.post("/projects", projects::create_project, "create_project");
    router.delete("/projects/:id", projects::remove_project, "remove_project");

    router.get(
        "/projects/:id/repository/tree",
        repository::show_tree,
        "show_tree",
    );

    router.get("/users", users::get_users, "users/get_users");
    router.get("/users/:id", users::get_user, "users/get_user");
    router.post("/users", users::create_user, "users/create_user");

    let mut chain = Chain::new(router);
    chain.link_after(JsonResponseMiddleware::new());

    chain
}
