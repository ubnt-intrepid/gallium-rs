use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use bodyparser::Struct;
use error::AppError;
use crypto::generate_sha1_hash;
use diesel::prelude::*;
use diesel::insert;

use app::App;
use models::{Application, NewApplication};
use schema::applications;

#[derive(Serialize)]
pub struct EncodableApplication {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub client_id: String,
}

impl From<Application> for EncodableApplication {
    fn from(app: Application) -> Self {
        EncodableApplication {
            id: app.id,
            name: app.name,
            created_at: app.created_at.format("%c").to_string(),
            client_id: app.client_id,
        }
    }
}

pub(super) fn get_app_list(req: &mut Request) -> IronResult<Response> {
    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let apps: Vec<_> = applications::table
        .load::<Application>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(EncodableApplication::from)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(apps))))
}

pub(super) fn register_app(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        name: String,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(AppError::from(""), status::BadRequest))?;

    let client_id = generate_sha1_hash();
    let new_app = NewApplication {
        name: &params.name,
        client_id: &client_id,
    };
    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let oauth_app: EncodableApplication = insert(&new_app)
        .into(applications::table)
        .get_result::<Application>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into();

    Ok(Response::with(
        (status::Created, JsonResponse::json(oauth_app)),
    ))
}
