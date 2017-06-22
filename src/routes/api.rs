use std::env;
use std::error;
use std::fmt;
use iron::prelude::*;
use iron::status;
use iron::headers::ContentType;
use iron::modifiers::Header;
use router::Router;
use bodyparser::Struct;
use serde_json;
use models::EncodablePublicKey;

#[derive(Debug)]
pub struct ApiError;

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "API error")
    }
}

impl error::Error for ApiError {
    fn description(&self) -> &str {
        "API error"
    }
}


fn handle_get_ssh_keys(_req: &mut Request) -> IronResult<Response> {
    use diesel::prelude::*;
    use diesel::pg::PgConnection;
    use models::PublicKey;
    use schema::public_keys;

    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    let keys: Vec<EncodablePublicKey> = public_keys::table
        .load::<PublicKey>(&conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((
        status::Ok,
        Header(ContentType::json()),
        serde_json::to_string_pretty(&keys).unwrap(),
    )))
}

fn handle_add_ssh_key(req: &mut Request) -> IronResult<Response> {
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
                Header(ContentType::json()),
                serde_json::to_string_pretty(&json!({
                    "message": "Failed to parse parameter as JSON",
                })).unwrap(),
            ))
        })?;

    use diesel::insert;
    use diesel::prelude::*;
    use diesel::pg::PgConnection;
    use schema::public_keys;
    use models::{PublicKey, NewPublicKey};

    let new_key = NewPublicKey {
        user_id: params.user_id,
        title: &params.title,
        key: &params.key,
    };
    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    let inserted_key: EncodablePublicKey = insert(&new_key)
        .into(public_keys::table)
        .get_result::<PublicKey>(&conn)
        .map(Into::into)
        .map_err(|err| {
            let err_message = format!("Failed to insert requested key: {}", err);
            IronError::new(err, (
                status::InternalServerError,
                Header(ContentType::json()),
                serde_json::to_string_pretty(&json!({
                    "message": err_message,
                })).unwrap(),
            ))
        })?;

    Ok(Response::with((
        status::Created,
        Header(ContentType::json()),
        serde_json::to_string_pretty(&inserted_key).unwrap(),
    )))
}

pub fn create_api_handler() -> Router {
    let mut router = Router::new();
    router.get("/user/keys", handle_get_ssh_keys, "get_ssh_keys");
    router.post("/user/keys", handle_add_ssh_key, "add_ssh_key");

    router
}
