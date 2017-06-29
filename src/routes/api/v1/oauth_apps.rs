use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use bodyparser::Struct;
use router::Router;
use error::AppError;
use diesel::prelude::*;
use diesel::delete;
use db::DB;

use models::OAuthApp;
use schema::apps;

#[derive(Serialize)]
pub struct EncodableApplication {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub client_id: String,
    pub client_secret: String,
}

impl From<OAuthApp> for EncodableApplication {
    fn from(app: OAuthApp) -> Self {
        EncodableApplication {
            id: app.id,
            name: app.name,
            created_at: app.created_at.format("%c").to_string(),
            client_id: app.client_id,
            client_secret: app.client_secret,
        }
    }
}


pub(super) fn get_app_list(req: &mut Request) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let apps: Vec<_> = OAuthApp::load_apps(db)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(EncodableApplication::from)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(apps))))
}

pub(super) fn get_client(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let app: EncodableApplication = OAuthApp::find_by_id(db, id)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?
        .into();

    Ok(Response::with((status::Ok, JsonResponse::json(app))))
}

pub(super) fn register_app(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        name: String,
        user_id: i32,
        redirect_uri: Option<String>,
    }
    let Params {
        name,
        user_id,
        redirect_uri,
    } = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(AppError::from(""), status::BadRequest))?;
    let redirect_uri = redirect_uri.as_ref().map(|s| s.as_str());

    let db = req.extensions.get::<DB>().unwrap();
    let oauth_app: EncodableApplication = OAuthApp::create(db, &name, user_id, redirect_uri)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into();

    Ok(Response::with(
        (status::Created, JsonResponse::json(oauth_app)),
    ))
}

pub(super) fn delete_client(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    delete(apps::table.filter(apps::dsl::id.eq(id)))
        .execute(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with(status::NoContent))
}
