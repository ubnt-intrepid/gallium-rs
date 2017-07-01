use diesel::delete;
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::JsonResponse;

use models::Project;
use models::repository;
use schema::projects;

use db::DB;
use config::Config;
use super::error;



#[derive(Route)]
#[get(path = "/projects", handler = "get_projects")]
pub(super) struct GetProjects;

fn get_projects(req: &mut Request) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;

    let repos: Vec<EncodableProject> = projects::table
        .load::<Project>(&*conn)
        .map_err(error::server_error)?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(repos))))
}



#[derive(Route)]
#[get(path = "/projects/:id", handler = "get_project")]
pub(super) struct GetProject;

fn get_project(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;

    let repo: EncodableProject = projects::table
        .filter(projects::dsl::id.eq(id))
        .get_result::<Project>(&*conn)
        .map_err(error::server_error)?
        .into();

    Ok(Response::with((status::Ok, JsonResponse::json(repo))))
}


#[derive(Route)]
#[post(path = "/projects", handler = "create_project")]
pub(super) struct CreateProject;

fn create_project(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        user: String,
        name: String,
        description: Option<String>,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| error::bad_request(""))?;

    let db = req.extensions.get::<DB>().unwrap();
    let config = req.extensions.get::<Config>().unwrap();

    let project: EncodableProject = Project::create(
        db,
        config,
        &params.user,
        &params.name,
        params.description.as_ref().map(|s| s.as_str()),
    ).map_err(error::server_error)?
        .into();

    Ok(Response::with(
        (status::Created, JsonResponse::json(project)),
    ))
}



#[derive(Route)]
#[delete(path = "/projects/:id", handler = "delete_project")]
pub(super) struct DeleteProject;

fn delete_project(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let config = req.extensions.get::<Config>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;

    let result = repository::open_repository_from_id(db, config, id)
        .map_err(error::server_error)?;
    let (_, _, repo) = match result {
        Some(r) => r,
        None => return Ok(Response::with(status::Ok)),
    };

    repo.remove().map_err(|(_, err)| {
        IronError::new(err, status::InternalServerError)
    })?;

    delete(projects::table.filter(projects::dsl::id.eq(id)))
        .execute(&*conn)
        .map_err(error::server_error)?;

    Ok(Response::with(
        (status::NoContent, JsonResponse::json(json!({}))),
    ))
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
