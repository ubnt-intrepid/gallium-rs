use diesel::delete;
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;

use models::{Project, NewProject};

use db::DB;
use super::{response, error};



#[derive(Route)]
#[get(path = "/projects", handler = "get_projects")]
pub(super) struct GetProjects;

fn get_projects(req: &mut Request) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;

    use schema::projects;
    let repos: Vec<EncodableProject> = projects::table
        .load::<Project>(&*conn)
        .map_err(error::server_error)?
        .into_iter()
        .map(Into::into)
        .collect();

    response::ok(repos)
}



#[derive(Route)]
#[get(path = "/projects/:id", handler = "get_project")]
pub(super) struct GetProject;

fn get_project(req: &mut Request, id: i32) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;

    use schema::projects;
    let repo: EncodableProject = projects::table
        .filter(projects::dsl::id.eq(id))
        .get_result::<Project>(&*conn)
        .map_err(error::server_error)?
        .into();

    response::ok(repo)
}


#[derive(Route)]
#[post(path = "/projects", handler = "create_project")]
pub(super) struct CreateProject;

fn create_project(req: &mut Request) -> IronResult<Response> {
    let new_project = req.get::<Struct<NewProject>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| error::bad_request(""))?;

    let db = req.extensions.get::<DB>().unwrap();
    let project = new_project.insert(db).map_err(error::server_error)?;

    response::created(EncodableProject::from(project))
}



#[derive(Route)]
#[delete(path = "/projects/:id", handler = "delete_project")]
pub(super) struct DeleteProject;

fn delete_project(req: &mut Request, id: i32) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;

    let project: Project = match Project::find_by_id(&db, id).map_err(error::server_error)? {
        Some(p) => p,
        None => return Ok(Response::with(status::Ok)),
    };
    let repo = project.open_repository(&*conn).map_err(error::server_error)?;

    repo.remove().map_err(|(_, err)| {
        IronError::new(err, status::InternalServerError)
    })?;

    let conn = db.get_db_conn().map_err(error::server_error)?;
    delete(::schema::projects::table.filter(
        ::schema::projects::dsl::id.eq(id),
    )).execute(&*conn)
        .map_err(error::server_error)?;

    response::no_content()
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
