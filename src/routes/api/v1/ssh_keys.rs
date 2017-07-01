use diesel::{insert, delete};
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::JsonResponse;

use models::{SshKey, NewSshKey};
use schema::ssh_keys;
use db::DB;
use super::error;


#[derive(Route)]
#[get(path = "/ssh_keys", handler = "get_ssh_keys")]
pub(super) struct GetKeys;

fn get_ssh_keys(req: &mut Request) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;
    let keys: Vec<EncodablePublicKey> = ssh_keys::table
        .load::<SshKey>(&*conn)
        .map_err(error::server_error)?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(&keys))))
}



#[derive(Route)]
#[get(path = "/ssh_keys/:id", handler = "get_ssh_key")]
pub(super) struct GetKey;

fn get_ssh_key(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;
    let key: EncodablePublicKey = ssh_keys::table
        .filter(ssh_keys::dsl::id.eq(id))
        .get_result::<SshKey>(&*conn)
        .map_err(error::server_error)?
        .into();

    Ok(Response::with((status::Ok, JsonResponse::json(&key))))
}



#[derive(Route)]
#[post(path = "/ssh_keys", handler = "add_ssh_key")]
pub(super) struct AddKey;

fn add_ssh_key(req: &mut Request) -> IronResult<Response> {
    let new_key = req.get::<Struct<NewSshKey>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| error::bad_request(""))?;

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;
    let inserted_key: EncodablePublicKey = insert(&new_key)
        .into(ssh_keys::table)
        .get_result::<SshKey>(&*conn)
        .map(Into::into)
        .map_err(error::server_error)?;

    Ok(Response::with(
        (status::Created, JsonResponse::json(&inserted_key)),
    ))
}



#[derive(Route)]
#[delete(path = "/ssh_keys/:id", handler = "delete_ssh_key")]
pub(super) struct DeleteKey;

fn delete_ssh_key(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let conn = db.get_db_conn().map_err(error::server_error)?;

    delete(ssh_keys::table.filter(ssh_keys::dsl::id.eq(id)))
        .execute(&*conn)
        .map_err(error::server_error)?;

    Ok(Response::with(
        (status::NoContent, JsonResponse::json(json!({}))),
    ))
}



#[derive(Serialize)]
pub struct EncodablePublicKey {
    id: i32,
    created_at: String,
    user_id: i32,
    description: Option<String>,
    key: String,
}

impl From<SshKey> for EncodablePublicKey {
    fn from(val: SshKey) -> Self {
        EncodablePublicKey {
            id: val.id,
            created_at: val.created_at.format("%c").to_string(),
            user_id: val.user_id,
            description: val.description,
            key: val.key,
        }
    }
}
