use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;
use error::AppError;

use db::DB;
use super::error;
use models::Project;

#[derive(Route)]
#[get(path = "/projects/:id/repository/tree", handler = "show_tree")]
pub(super) struct ShowTree;

// TODO: use `git ls-tree`
fn show_tree(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let project = Project::find_by_id(&db, id)
        .map_err(error::server_error)?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?;
    let repo = project.open_repository(&db).map_err(error::server_error)?;

    let tree = repo.get_head_tree_objects().map_err(error::server_error)?;

    Ok(Response::with((status::Ok, JsonResponse::json(tree))))
}
