use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use iron_json_response::JsonResponse;
use diesel::insert;
use diesel::prelude::*;
use schema::public_keys;
use models::{PublicKey, NewPublicKey, EncodablePublicKey};
use super::ApiError;
use app::App;

pub(super) fn handle_get_ssh_keys(req: &mut Request) -> IronResult<Response> {
    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, (
            status::InternalServerError,
            JsonResponse::json(json!({
                "message": "Failed to retrieve pooled DB connection",
            })),
        ))
    })?;
    let keys: Vec<EncodablePublicKey> = public_keys::table
        .load::<PublicKey>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(&keys))))
}

pub(super) fn handle_add_ssh_key(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        user_id: i32,
        title: String,
        key: String,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| {
            IronError::new(ApiError, (
                status::BadRequest,
                JsonResponse::json(&json!({
                    "message": "Failed to parse parameter as JSON",
                })),
            ))
        })?;

    let new_key = NewPublicKey {
        user_id: params.user_id,
        title: &params.title,
        key: &params.key,
    };

    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, (
            status::InternalServerError,
            JsonResponse::json(json!({
                "message": "Failed to retrieve pooled DB connection",
            })),
        ))
    })?;
    let inserted_key: EncodablePublicKey = insert(&new_key)
        .into(public_keys::table)
        .get_result::<PublicKey>(&*conn)
        .map(Into::into)
        .map_err(|err| {
            let err_message = format!("Failed to insert requested key: {}", err);
            IronError::new(err, (
                status::InternalServerError,
                JsonResponse::json(&json!({
                    "message": err_message,
                })),
            ))
        })?;

    Ok(Response::with(
        (status::Created, JsonResponse::json(&inserted_key)),
    ))
}
