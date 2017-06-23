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
use std::io;
use std::path::Path;
use std::process::Command;
use std::os::unix::process::CommandExt;
use users::get_user_by_name;

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
    create_repository(&repo_path).map_err(|err| {
        IronError::new(err, (
            status::InternalServerError,
            JsonResponse::json(json!({
                "message": "failed to create Git repository",
            })),
        ))
    })?;

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_project)),
    ))
}

fn create_repository<P: AsRef<Path>>(repo_path: P) -> io::Result<()> {
    let repo_path_str = repo_path.as_ref().to_str().unwrap();

    // Get uid/gid
    let user = get_user_by_name("git").unwrap();
    let uid = user.uid();
    let gid = user.primary_group_id();

    // Create destination directory of repository.
    // TODO: use libc
    Command::new("/bin/mkdir")
        .args(&["-p", repo_path_str])
        .uid(uid)
        .gid(gid)
        .spawn()
        .and_then(|mut ch| ch.wait())
        .and_then(|st| if st.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "cannot change owner of repository",
            ))
        })?;

    // Initialize git repository
    Command::new("/usr/bin/git")
        .args(&["init", "--bare", repo_path_str])
        .current_dir(&repo_path)
        .uid(uid)
        .gid(gid)
        .spawn()
        .and_then(|mut ch| ch.wait())
        .and_then(|status| if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "`git init` exited with non-zero status",
            ))
        })
}
