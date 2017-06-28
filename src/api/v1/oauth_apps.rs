use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use bodyparser::Struct;
use router::Router;
use error::AppError;
use crypto;
use diesel::prelude::*;
use diesel::insert;
use diesel::delete;
use db::DB;

use models::{OAuthApp, NewOAuthApp};
use schema::oauth_apps;

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
    let conn = db.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let apps: Vec<_> = oauth_apps::table
        .load::<OAuthApp>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(EncodableApplication::from)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(apps))))
}

pub(super) fn get_client(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let client: EncodableApplication = oauth_apps::table
        .filter(oauth_apps::dsl::id.eq(id))
        .get_result::<OAuthApp>(&*conn)
        .optional()
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?
        .into();
    Ok(Response::with((status::Ok, JsonResponse::json(client))))
}

pub(super) fn delete_client(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    delete(oauth_apps::table.filter(oauth_apps::dsl::id.eq(id)))
        .execute(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with(status::NoContent))
}

pub(super) fn register_app(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        name: String,
        user_id: i32,
        redirect_uri: Option<String>,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(AppError::from(""), status::BadRequest))?;

    let client_id = crypto::generate_sha1_hash();
    let client_secret = crypto::generate_sha1_random();
    let new_app = NewOAuthApp {
        name: &params.name,
        user_id: params.user_id,
        client_id: &client_id,
        client_secret: &client_secret,
        redirect_uri: params.redirect_uri.as_ref().map(|s| s.as_str()),
    };
    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let oauth_app: EncodableApplication = insert(&new_app)
        .into(oauth_apps::table)
        .get_result::<OAuthApp>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into();

    Ok(Response::with(
        (status::Created, JsonResponse::json(oauth_app)),
    ))
}
