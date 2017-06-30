use diesel::{insert, delete};
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::{JsonResponse, JsonResponseMiddleware};

use error::{AppResult, AppError};
use models::{Project, NewProject, Repository};
use models::repository;
use schema::{users, projects};

use db::DB;
use config::Config;


pub(super) fn create_routes() -> Chain {
    let mut router = Router::new();
    router.get("/", get_projecs, "get_projects");
    router.get("/:id", get_project, "get_project");
    router.post("/", create_project, "create_project");
    router.delete("/:id", remove_project, "remove_project");

    router.get(
        "/:id/repository/tree",
        super::repository::show_tree,
        "show_tree",
    );

    let mut chain = Chain::new(router);
    chain.link_after(JsonResponseMiddleware::new());
    chain
}


#[derive(Debug, Serialize)]
pub struct EncodableProject {
    pub id: i32,
    pub created_at: String,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
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
    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(|err| {
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

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(|err| {
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
        name: String,
        description: Option<String>,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| {
            IronError::new(AppError::from("Invalid Request"), status::BadRequest)
        })?;

    let db = req.extensions.get::<DB>().unwrap();
    let config = req.extensions.get::<Config>().unwrap();
    if repository::open_repository(db, config, &params.user, &params.name).is_ok() {
        return Err(IronError::new(
            AppError::from("The repository has already created."),
            status::Conflict,
        ));
    }

    create_new_repository(config, &params.user, &params.name)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let conn = db.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let user_id: i32 = users::table
        .filter(users::dsl::name.eq(&params.user))
        .select(users::dsl::id)
        .get_result(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let new_project = NewProject {
        name: &params.name,
        user_id,
        description: params.description.as_ref().map(|s| s.as_str()),
    };
    let inserted_project = insert(&new_project)
        .into(projects::table)
        .get_result::<Project>(&*conn)
        .map(EncodableProject::from)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_project)),
    ))
}

pub(super) fn remove_project(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let config = req.extensions.get::<Config>().unwrap();
    let conn = db.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let result = repository::open_repository_from_id(db, config, id)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;
    let (_, _, repo) = match result {
        Some(r) => r,
        None => return Ok(Response::with(status::Ok)),
    };

    repo.remove().map_err(|(_, err)| {
        IronError::new(err, status::InternalServerError)
    })?;

    delete(projects::table.filter(projects::dsl::id.eq(id)))
        .execute(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with(
        (status::NoContent, JsonResponse::json(json!({}))),
    ))
}


fn create_new_repository(config: &Config, user: &str, project: &str) -> AppResult<()> {
    let repo_path = config.repository_path(user, project);
    Repository::create(&repo_path)?;
    Ok(())
}
