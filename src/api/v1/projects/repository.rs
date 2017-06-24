use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;
use git2::{Repository, Commit, ObjectType, Tree};
use serde_json::Value;
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

    let tree = head_commit.tree().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let elems = collect_tree_object(&repo, &tree);

    Ok(Response::with((status::Ok, JsonResponse::json(elems))))
}

fn collect_tree_object(repo: &Repository, tree: &Tree) -> Vec<Value> {
    tree.into_iter()
        .map(|entry| {
            let kind = entry.kind().unwrap();
            match kind {
                ObjectType::Blob => {
                    json!({
                        "name": entry.name().unwrap(),
                        "filemode": format!("{:06o}", entry.filemode()),
                    })
                }
                ObjectType::Tree => {
                    let child = collect_tree_object(
                        repo,
                        &entry
                            .to_object(repo)
                            .map(|o| o.into_tree().ok().unwrap())
                            .unwrap(),
                    );
                    json!({
                        "name": entry.name().unwrap(),
                        "filemode": format!("{:06o}", entry.filemode()),
                        "child": child,
                    })
                }
                _ => Default::default(),
            }
        })
        .collect()
}
