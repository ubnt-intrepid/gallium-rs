use diesel::delete;
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::{JsonResponse, JsonResponseMiddleware};

use error::AppError;
use models::Project;
use models::repository;
use schema::projects;

use db::DB;
use config::Config;

use super::Route;
use iron::method::Method;
use iron::Handler;


pub(super) struct GetProjects;

impl Route for GetProjects {
    fn route_id() -> &'static str {
        "get_projects"
    }
    fn route_method() -> Method {
        Method::Get
    }
    fn route_path() -> &'static str {
        "/projects"
    }
}

impl Into<Chain> for GetProjects {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for GetProjects {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
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
}



pub(super) struct GetProject;

impl Route for GetProject {
    fn route_id() -> &'static str {
        "get_project"
    }
    fn route_method() -> Method {
        Method::Get
    }
    fn route_path() -> &'static str {
        "/projects/:id"
    }
}

impl Into<Chain> for GetProject {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for GetProject {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
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
}


pub(super) struct CreateProject;

impl Route for CreateProject {
    fn route_id() -> &'static str {
        "create_project"
    }
    fn route_method() -> Method {
        Method::Post
    }
    fn route_path() -> &'static str {
        "/projects"
    }
}

impl Into<Chain> for CreateProject {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for CreateProject {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
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

        let project = Project::create(
            db,
            config,
            &params.user,
            &params.name,
            params.description.as_ref().map(|s| s.as_str()),
        ).map(EncodableProject::from)
            .map_err(|err| IronError::new(err, status::InternalServerError))?;

        Ok(Response::with(
            (status::Created, JsonResponse::json(project)),
        ))
    }
}


pub(super) struct DeleteProject;

impl Route for DeleteProject {
    fn route_id() -> &'static str {
        "delete_project"
    }
    fn route_method() -> Method {
        Method::Delete
    }
    fn route_path() -> &'static str {
        "/projects/:id"
    }
}

impl Into<Chain> for DeleteProject {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for DeleteProject {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
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
