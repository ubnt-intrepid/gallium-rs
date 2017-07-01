use iron::prelude::*;
use iron::Handler;
use iron::status;
use iron_json_response::{JsonResponse, JsonResponseMiddleware};
use router::Router;
use error::AppError;

use db::DB;
use config::Config;
use models::repository;


#[derive(Route)]
#[get(path = "/projects/:id/repository/tree")]
pub(super) struct ShowTree;

impl Into<Chain> for ShowTree {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for ShowTree {
    // TODO: use `git ls-tree`
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let router = req.extensions.get::<Router>().unwrap();
        let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

        let db = req.extensions.get::<DB>().unwrap();
        let config = req.extensions.get::<Config>().unwrap();
        let (_, _, repo) = repository::open_repository_from_id(db, config, id)
            .map_err(|err| IronError::new(err, status::InternalServerError))?
            .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?;

        let tree = repo.get_head_tree_objects().map_err(|err| {
            IronError::new(err, status::InternalServerError)
        })?;

        Ok(Response::with((status::Ok, JsonResponse::json(tree))))
    }
}
