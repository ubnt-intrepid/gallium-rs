use diesel::{insert, delete};
use diesel::prelude::*;
use iron::prelude::*;
use iron::Handler;
use iron::method::Method;
use iron::status;
use bodyparser::Struct;
use router::Router;
use iron_json_response::{JsonResponse, JsonResponseMiddleware};

use error::AppError;
use models::{SshKey, NewSshKey};
use schema::ssh_keys;
use db::DB;
use super::Route;


pub(super) struct GetKeys;

impl Route for GetKeys {
    fn route_id() -> &'static str {
        "get_keys"
    }
    fn route_method() -> Method {
        Method::Get
    }
    fn route_path() -> &'static str {
        "/ssh_keys"
    }
}

impl Into<Chain> for GetKeys {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain

    }
}

impl Handler for GetKeys {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let db = req.extensions.get::<DB>().unwrap();
        let conn = db.get_db_conn().map_err(|err| {
            IronError::new(err, status::InternalServerError)
        })?;
        let keys: Vec<EncodablePublicKey> = ssh_keys::table
            .load::<SshKey>(&*conn)
            .map_err(|err| IronError::new(err, status::InternalServerError))?
            .into_iter()
            .map(Into::into)
            .collect();

        Ok(Response::with((status::Ok, JsonResponse::json(&keys))))
    }
}



pub(super) struct GetKey;

impl Route for GetKey {
    fn route_id() -> &'static str {
        "get_key"
    }
    fn route_method() -> Method {
        Method::Get
    }
    fn route_path() -> &'static str {
        "/ssh_keys/:id"
    }
}

impl Into<Chain> for GetKey {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for GetKey {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let router = req.extensions.get::<Router>().unwrap();
        let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

        let db = req.extensions.get::<DB>().unwrap();
        let conn = db.get_db_conn().map_err(|err| {
            IronError::new(err, status::InternalServerError)
        })?;
        let key: EncodablePublicKey = ssh_keys::table
            .filter(ssh_keys::dsl::id.eq(id))
            .get_result::<SshKey>(&*conn)
            .map_err(|err| IronError::new(err, status::NotFound))?
            .into();

        Ok(Response::with((status::Ok, JsonResponse::json(&key))))
    }
}



pub(super) struct AddKey;

impl Route for AddKey {
    fn route_id() -> &'static str {
        "add_key"
    }
    fn route_method() -> Method {
        Method::Post
    }
    fn route_path() -> &'static str {
        "/ssh_keys"
    }
}

impl Into<Chain> for AddKey {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for AddKey {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let new_key = req.get::<Struct<NewSshKey>>()
            .ok()
            .and_then(|s| s)
            .ok_or_else(|| IronError::new(AppError::from(""), status::BadRequest))?;

        let db = req.extensions.get::<DB>().unwrap();
        let conn = db.get_db_conn().map_err(|err| {
            IronError::new(err, status::InternalServerError)
        })?;
        let inserted_key: EncodablePublicKey = insert(&new_key)
            .into(ssh_keys::table)
            .get_result::<SshKey>(&*conn)
            .map(Into::into)
            .map_err(|err| IronError::new(err, status::InternalServerError))?;

        Ok(Response::with(
            (status::Created, JsonResponse::json(&inserted_key)),
        ))
    }
}



pub(super) struct DeleteKey;

impl Route for DeleteKey {
    fn route_id() -> &'static str {
        "delete_key"
    }
    fn route_method() -> Method {
        Method::Delete
    }
    fn route_path() -> &'static str {
        "/ssh_keys/:id"
    }
}

impl Into<Chain> for DeleteKey {
    fn into(self) -> Chain {
        let mut chain = Chain::new(self);
        chain.link_after(JsonResponseMiddleware::new());
        chain
    }
}

impl Handler for DeleteKey {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let router = req.extensions.get::<Router>().unwrap();
        let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

        let db = req.extensions.get::<DB>().unwrap();
        let conn = db.get_db_conn().map_err(|err| {
            IronError::new(err, status::InternalServerError)
        })?;

        delete(ssh_keys::table.filter(ssh_keys::dsl::id.eq(id)))
            .execute(&*conn)
            .map_err(|err| IronError::new(err, status::InternalServerError))?;

        Ok(Response::with(
            (status::NoContent, JsonResponse::json(json!({}))),
        ))
    }
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
