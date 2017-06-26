use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;
use error::AppError;
use app::App;

// TODO: use `git ls-tree`
pub fn show_tree(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let app: &App = req.extensions.get::<App>().unwrap();
    let (_, _, repo) = app.open_repository_from_id(id)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?;

    let tree = repo.get_head_tree_objects().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    Ok(Response::with((status::Ok, JsonResponse::json(tree))))
}
