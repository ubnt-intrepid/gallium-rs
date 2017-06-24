use iron::prelude::*;
use iron::status;
use router::Router;
use git2::{Repository, Commit};
use diesel::prelude::*;
use app::App;
use models::{User, Project};
use schema::{users, projects};
use api::ApiError;

pub fn get_file_list(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let project: Project = projects::table
        .filter(projects::dsl::id.eq(id))
        .get_result(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let user: User = users::table
        .filter(users::dsl::id.eq(project.user_id))
        .get_result(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let repo_path = app.resolve_repository_path(&user.username, &format!("{}.git", project.name))
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let repo: Repository = Repository::open(&repo_path).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let head_ref = repo.head().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let target_oid = head_ref.target().ok_or_else(|| {
        IronError::new(ApiError(""), status::InternalServerError)
    })?;

    let head_commit: Commit = repo.find_commit(target_oid).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    Ok(Response::with((status::Ok, head_commit.message().unwrap())))
}
