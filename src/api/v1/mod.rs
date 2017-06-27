mod auth;
mod error;
mod keys;
mod projects;
mod repository;
mod users;

use iron::prelude::*;
use router::Router;
use iron_json_response::JsonResponseMiddleware;
use middleware::authentication::AuthMiddleware;

macro_rules! v {
    ($i:expr) => {
        {
            let mut chain = Chain::new($i);
            chain.link_before(AuthMiddleware);
            chain
        }
    }
}

pub fn create_api_handler() -> Chain {
    let mut router = Router::new();

    router.post("/oauth/token", auth::token_endpoint, "auth/token_endpoint");

    router.get("/keys", v!(keys::get_keys), "keys/get_keys");
    router.get("/keys/:id", v!(keys::get_key), "keys/get_key");
    router.post("/keys", v!(keys::add_key), "keys/add_key");
    router.delete("/keys/:id", v!(keys::delete_key), "keys/delete_key");

    router.get("/projects", v!(projects::get_projecs), "get_projects");
    router.get("/projects/:id", v!(projects::get_project), "get_project");
    router.post("/projects", v!(projects::create_project), "create_project");
    router.delete(
        "/projects/:id",
        v!(projects::remove_project),
        "remove_project",
    );

    router.get(
        "/projects/:id/repository/tree",
        v!(repository::show_tree),
        "show_tree",
    );

    router.get("/users", v!(users::get_users), "users/get_users");
    router.get("/users/:id", v!(users::get_user), "users/get_user");
    router.post("/users", v!(users::create_user), "users/create_user");

    let mut chain = Chain::new(router);
    chain.link_after(JsonResponseMiddleware::new());

    chain
}
