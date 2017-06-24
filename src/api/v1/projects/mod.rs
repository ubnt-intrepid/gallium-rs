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
use git;
use models::{User, Project, NewProject};
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
    if app.get_repository(&params.user, &params.project).is_ok() {
        return Err(IronError::new(ApiError(""), status::Conflict));
    }

    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let user_id: i32 = users::table
        .filter(users::dsl::name.eq(&params.user))
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
    git::Repository::create(&repo_path).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_project)),
    ))
}


trait GetProjectInfoFromId {
    fn get_project_from_id(&self) -> IronResult<(User, Project)>;
}

impl<'a, 'b: 'a> GetProjectInfoFromId for Request<'a, 'b> {
    fn get_project_from_id(&self) -> IronResult<(User, Project)> {
        let router = self.extensions.get::<Router>().unwrap();
        let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

        let app = self.extensions.get::<App>().unwrap();
        let conn = app.get_db_conn().map_err(|err| {
            IronError::new(err, status::InternalServerError)
        })?;
        users::table
            .inner_join(projects::table)
            .filter(projects::dsl::id.eq(id))
            .get_result::<(User, Project)>(&*conn)
            .optional()
            .map_err(|err| IronError::new(err, status::InternalServerError))
            .and_then(|result| {
                result.ok_or_else(|| IronError::new(ApiError(""), status::NotFound))
            })
    }
}
