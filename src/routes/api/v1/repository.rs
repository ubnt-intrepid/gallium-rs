use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;
use error::AppError;
use url::Url;
use std::borrow::Borrow;
use db::DB;
use super::error;
use models::Project;

#[derive(Route)]
#[get(path = "/projects/:id/repository/tree", handler = "show_tree")]
pub(super) struct ShowTree;

fn show_tree(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let (mut refname, mut path, mut recursive) = (None, None, None);
    let url: Url = req.url.clone().into();
    for (key, val) in url.query_pairs() {
        match key.borrow() {
            "ref" => refname = Some(val.into_owned()),
            "path" => path = Some(val.into_owned()),
            "recursive" => recursive = val.parse().ok(),
            _ => (),
        }
    }

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;
    let project = Project::find_by_id(&db, id)
        .map_err(error::server_error)?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?;
    let repo = project.open_repository(&*conn).map_err(error::server_error)?;

    let tree = repo.list_tree(
        refname.as_ref().map(|s| s.as_str()).unwrap_or("HEAD"),
        path.as_ref().map(|s| s.as_str()),
        recursive.unwrap_or(false),
    ).map_err(error::server_error)?;

    Ok(Response::with((status::Ok, JsonResponse::json(tree))))
}
