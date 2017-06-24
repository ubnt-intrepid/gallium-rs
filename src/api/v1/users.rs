use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::JsonResponse;
use bcrypt;
use api::ApiError;

use diesel::insert;
use diesel::prelude::*;
use models::{User, NewUser, EncodableUser};
use schema::users;

use app::App;

pub(super) fn get_users(req: &mut Request) -> IronResult<Response> {
    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let users: Vec<EncodableUser> = users::table
        .load::<User>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(users))))
}

pub(super) fn get_user(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let user: EncodableUser = users::table
        .filter(users::dsl::id.eq(id))
        .get_result::<User>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into();

    Ok(Response::with((status::Ok, JsonResponse::json(user))))
}

pub(super) fn create_user(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        username: String,
        email_address: String,
        password: String,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(ApiError(""), status::BadRequest))?;

    let bcrypt_hash = bcrypt::hash(&params.password, bcrypt::DEFAULT_COST)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let new_user = NewUser {
        username: &params.username,
        email_address: &params.email_address,
        bcrypt_hash: &bcrypt_hash,
    };

    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let inserted_user: EncodableUser = insert(&new_user)
        .into(users::table)
        .get_result::<User>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into();

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_user)),
    ))
}
