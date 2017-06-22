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


#[derive(Serialize)]
struct SerdePublicKey {
    id: i32,
    created_at: String,
    title: String,
    key: String,
}

impl From<super::models::PublicKey> for SerdePublicKey {
    fn from(val: super::models::PublicKey) -> SerdePublicKey {
        SerdePublicKey {
            id: val.id,
            created_at: val.created_at.format("%c").to_string(),
            title: val.title,
            key: val.key,
        }
    }
}

fn handle_get_ssh_keys(_req: &mut Request) -> IronResult<Response> {
    use diesel::prelude::*;
    use diesel::pg::PgConnection;
    use super::models::PublicKey;
    use super::schema::public_keys;

    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    let keys: Vec<SerdePublicKey> = public_keys::table
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
    use super::schema::public_keys;
    use super::models::{PublicKey, NewPublicKey};

    let new_key = NewPublicKey {
        user_id: params.user_id,
        title: &params.title,
        key: &params.key,
    };
    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    let inserted_key: SerdePublicKey = insert(&new_key)
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
    let mut api_router = Router::new();
    api_router.get("/user/keys", handle_get_ssh_keys, "get_ssh_keys");
    api_router.post("/user/keys", handle_add_ssh_key, "add_ssh_key");

    api_router
}
