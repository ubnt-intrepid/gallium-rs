use iron::prelude::*;
use iron::status;
use std::borrow::Borrow;
use iron::headers::ContentType;
use iron::modifiers::Header;
use base64;
use url::Url;

use db::DB;
use models::{Project, Repository};
use super::{response, error};


fn open_repository_from_id(req: &mut Request, id: i32) -> IronResult<Repository> {
    let conn = DB::from_req(req).map_err(error::server_error)?;
    let project = Project::find_by_id(&conn, id)
        .map_err(error::server_error)?
        .ok_or_else(|| error::not_found(""))?;
    project.open_repository(&*conn).map_err(error::server_error)
}


#[derive(Route)]
#[get(path = "/projects/:id/repository/tree", handler = "show_tree")]
pub(super) struct ShowTree;

fn show_tree(req: &mut Request, id: i32) -> IronResult<Response> {
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
    let refname = refname.as_ref().map(|s| s.as_str()).unwrap_or("HEAD");
    let path = path.as_ref().map(|s| s.as_str());
    let recursive = recursive.unwrap_or(false);

    let repo = open_repository_from_id(req, id)?;
    let tree = repo.list_tree(refname, path, recursive).map_err(
        error::server_error,
    )?;
    response::ok(tree)
}


#[derive(Route)]
#[get(path = "/projects/:id/repository/blobs/:sha", handler = "get_blob")]
pub(super) struct GetBlob;

fn get_blob(req: &mut Request, id: i32, sha: String) -> IronResult<Response> {
    let repo = open_repository_from_id(req, id)?;
    let content = repo.get_blob_content(&sha)
        .map_err(error::server_error)?
        .map(|content| base64::encode(&content))
        .ok_or_else(|| error::not_found(""))?;

    response::ok(json!({
        "sha": sha,
        "encoding": "base64",
        "content": content,
        "size": content.len(),
    }))
}


#[derive(Route)]
#[get(path = "/projects/:id/repository/blobs/:sha/raw", handler = "get_raw_blob")]
pub(super) struct GetRawBlob;

fn get_raw_blob(req: &mut Request, id: i32, sha: String) -> IronResult<Response> {
    let repo = open_repository_from_id(req, id)?;
    let content = repo.get_blob_content(&sha)
        .map_err(error::server_error)?
        .ok_or_else(|| error::not_found(""))?;
    Ok(Response::with(
        (status::Ok, Header(ContentType::plaintext()), content),
    ))
}
