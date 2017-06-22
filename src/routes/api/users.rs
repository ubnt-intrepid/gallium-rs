use std::env;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use iron_json_response::JsonResponse;
use bcrypt;
use super::ApiError;

use diesel::insert;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use models::{User, NewUser, EncodableUser};
use schema::users;

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
        .ok_or_else(|| {
            IronError::new(ApiError, (
                status::BadRequest,
                JsonResponse::json(json!({
                    "message": "",
                })),
            ))
        })?;

    let bcrypt_hash = bcrypt::hash(&params.password, bcrypt::DEFAULT_COST)
        .map_err(|err| {
            IronError::new(err, (
                status::InternalServerError,
                JsonResponse::json(json!({
                    "message": "",
                })),
            ))
        })?;

    let new_user = NewUser {
        username: &params.username,
        email_address: &params.email_address,
        bcrypt_hash: &bcrypt_hash,
    };
    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    let inserted_user: EncodableUser = insert(&new_user)
        .into(users::table)
        .get_result::<User>(&conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into();

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_user)),
    ))
}
