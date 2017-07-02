use iron::prelude::*;
use iron::status;
use iron::headers::ContentType;
use iron::modifiers::Header;
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


#[derive(Route)]
#[get(path = "/projects/:id/repository/blobs/:sha", handler = "get_blob")]
pub(super) struct GetBlob;

fn get_blob(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();
    let sha = router.find("sha").unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;
    let project = Project::find_by_id(&db, id)
        .map_err(error::server_error)?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?;
    let repo = project.open_repository(&*conn).map_err(error::server_error)?;
    let blob = repo.get_blob(sha)
        .map_err(error::server_error)?
        .ok_or_else(|| {
            IronError::new(AppError::from("repository"), (
                status::NotFound,
                JsonResponse::json(json!({
                    "error": "not_found",
                })),
            ))
        })?;

    Ok(Response::with((status::Ok, JsonResponse::json(blob))))
}


#[derive(Route)]
#[get(path = "/projects/:id/repository/blobs/:sha/raw", handler = "get_raw_blob")]
pub(super) struct GetRawBlob;

fn get_raw_blob(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();
    let sha = router.find("sha").unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;
    let project = Project::find_by_id(&db, id)
        .map_err(error::server_error)?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?;
    let repo = project.open_repository(&*conn).map_err(error::server_error)?;
    let blob = repo.get_blob_raw(sha)
        .map_err(error::server_error)?
        .ok_or_else(|| {
            IronError::new(AppError::from("repository"), (
                status::NotFound,
                JsonResponse::json(json!({
                    "error": "not_found",
                })),
            ))
        })?;

    Ok(Response::with(
        (status::Ok, Header(ContentType::plaintext()), blob),
    ))
}
