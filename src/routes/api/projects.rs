use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use iron_json_response::JsonResponse;
use app::App;
use super::ApiError;

use diesel::insert;
use diesel::prelude::*;
use models::{Project, NewProject, EncodableProject};
use schema::{users, projects};
use std::fs;
use std::process::{Command, Stdio};

pub(super) fn get_projecs(req: &mut Request) -> IronResult<Response> {
    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, (
            status::InternalServerError,
            JsonResponse::json(json!({
                "message": "Failed to get DB connection"
            })),
        ))
    })?;

    let repos: Vec<EncodableProject> = projects::table
        .load::<Project>(&*conn)
        .map_err(|err| {
            IronError::new(err, (
                status::InternalServerError,
                JsonResponse::json(json!({
                    "message": "Failed to get repository list"
                })),
            ))
        })?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(repos))))
}

pub(super) fn create_project(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        user: String,
        project: String,
        description: String,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(ApiError, status::BadRequest))?;

    let app: &App = req.extensions.get::<App>().unwrap();
    if app.resolve_repository_path(&params.user, &params.project)
        .is_ok()
    {
        return Err(IronError::new(ApiError, status::Conflict));
    }

    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, (
            status::InternalServerError,
            JsonResponse::json(json!({
                "message": "Failed to get DB connection"
            })),
        ))
    })?;

    let user_id: i32 = users::table
        .filter(users::dsl::username.eq(&params.user))
        .select(users::dsl::id)
        .get_result(&*conn)
        .map_err(|err| {
            IronError::new(err, (
                status::InternalServerError,
                JsonResponse::json(json!({
                    "message": "Failed to get user id"
                })),
            ))
        })?;

    let new_project = NewProject {
        name: &params.project,
        user_id,
        description: &params.description,
    };
    let inserted_project: EncodableProject = insert(&new_project)
        .into(projects::table)
        .get_result::<Project>(&*conn)
        .map(Into::into)
        .map_err(|err| {
            IronError::new(err, (
                status::InternalServerError,
                JsonResponse::json(json!({
                    "message": "failed to insert new project"
                })),
            ))
        })?;

    let repo_path = app.generate_repository_path(&params.user, &params.project);
    fs::create_dir_all(&repo_path).map_err(|err| {
        IronError::new(err, (
            status::InternalServerError,
            JsonResponse::json(json!({
                "message": "failed to create reposity directory",
            })),
        ))
    })?;
    let status = Command::new("/usr/bin/git")
        .arg("init")
        .current_dir(&repo_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|mut ch| ch.wait())
        .map_err(|err| {
            IronError::new(err, (
                status::InternalServerError,
                JsonResponse::json(json!({
                    "message": "failed to execute git init"
                })),
            ))
        })?;
    if !status.success() {
        return Err(IronError::new(ApiError, (
            status::InternalServerError,
            JsonResponse::json(json!({
                "message": "`git init` is exited with non-zero status"
            })),
        )));
    }

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_project)),
    ))
}
