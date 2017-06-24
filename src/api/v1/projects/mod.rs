pub(super) mod repository;

use diesel::insert;
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::JsonResponse;

use api::ApiError;
use app::App;
use models::{Project, NewProject};
use schema::{users, projects};


#[derive(Debug, Serialize)]
pub struct EncodableProject {
    pub id: i32,
    pub created_at: String,
    pub user_id: i32,
    pub name: String,
    pub description: String,
}

impl From<Project> for EncodableProject {
    fn from(val: Project) -> Self {
        EncodableProject {
            id: val.id,
            created_at: val.created_at.format("%c").to_string(),
            user_id: val.user_id,
            name: val.name,
            description: val.description,
        }
    }
}


pub(super) fn get_projecs(req: &mut Request) -> IronResult<Response> {
    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let repos: Vec<EncodableProject> = projects::table
        .load::<Project>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(repos))))
}

pub(super) fn get_project(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let repo: EncodableProject = projects::table
        .filter(projects::dsl::id.eq(id))
        .get_result::<Project>(&*conn)
        .map_err(|err| IronError::new(err, status::NotFound))?
        .into();

    Ok(Response::with((status::Ok, JsonResponse::json(repo))))
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
        .ok_or_else(|| IronError::new(ApiError(""), status::BadRequest))?;

    let app: &App = req.extensions.get::<App>().unwrap();
    if app.resolve_repository_path(&params.user, &params.project)
        .is_ok()
    {
        return Err(IronError::new(ApiError(""), status::Conflict));
    }

    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let user_id: i32 = users::table
        .filter(users::dsl::username.eq(&params.user))
        .select(users::dsl::id)
        .get_result(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let new_project = NewProject {
        name: &params.project,
        user_id,
        description: &params.description,
    };
    let inserted_project: EncodableProject =
        insert(&new_project)
            .into(projects::table)
            .get_result::<Project>(&*conn)
            .map(Into::into)
            .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let repo_path = app.generate_repository_path(&params.user, &params.project);
    util::create_repository(&repo_path).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_project)),
    ))
}


mod util {
    use std::io;
    use std::path::Path;
    use std::process::Command;
    use std::os::unix::process::CommandExt;
    use users::get_user_by_name;

    pub(super) fn create_repository<P: AsRef<Path>>(repo_path: P) -> io::Result<()> {
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
}
