use diesel::{insert, delete};
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::JsonResponse;

use error::AppError;
use app::App;
use models::{PublicKey, NewPublicKey};
use schema::public_keys;


#[derive(Serialize)]
pub struct EncodablePublicKey {
    id: i32,
    created_at: String,
    user_id: i32,
    title: String,
    key: String,
}

impl From<PublicKey> for EncodablePublicKey {
    fn from(val: PublicKey) -> Self {
        EncodablePublicKey {
            id: val.id,
            created_at: val.created_at.format("%c").to_string(),
            user_id: val.user_id,
            title: val.title,
            key: val.key,
        }
    }
}


pub(super) fn get_keys(req: &mut Request) -> IronResult<Response> {
    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let keys: Vec<EncodablePublicKey> = public_keys::table
        .load::<PublicKey>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(&keys))))
}

pub(super) fn get_key(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let key: EncodablePublicKey = public_keys::table
        .filter(public_keys::dsl::id.eq(id))
        .get_result::<PublicKey>(&*conn)
        .map_err(|err| IronError::new(err, status::NotFound))?
        .into();

    Ok(Response::with((status::Ok, JsonResponse::json(&key))))
}

pub(super) fn add_key(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        user_id: i32,
        title: String,
        key: String,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(AppError::from(""), status::BadRequest))?;

    let new_key = NewPublicKey {
        user_id: params.user_id,
        title: &params.title,
        key: &params.key,
    };

    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let inserted_key: EncodablePublicKey =
        insert(&new_key)
            .into(public_keys::table)
            .get_result::<PublicKey>(&*conn)
            .map(Into::into)
            .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with(
        (status::Created, JsonResponse::json(&inserted_key)),
    ))
}

pub(super) fn delete_key(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    delete(public_keys::table.filter(public_keys::dsl::id.eq(id)))
        .execute(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with(
        (status::NoContent, JsonResponse::json(json!({}))),
    ))
}
