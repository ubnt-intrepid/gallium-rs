use iron::{IronResult, Response};
use iron::headers::ContentType;
use iron::modifiers::Header;
use iron::status;
use serde::Serialize;
use serde_json;
use super::error;

fn success<T: Serialize>(status: status::Status, val: T) -> IronResult<Response> {
    let body = serde_json::to_string(&val).map_err(error::server_error)?;
    Ok(Response::with((status, Header(ContentType::json()), body)))
}

pub fn ok<T: Serialize>(val: T) -> IronResult<Response> {
    success(status::Ok, val)
}

pub fn created<T: Serialize>(val: T) -> IronResult<Response> {
    success(status::Created, val)
}

pub fn no_content() -> IronResult<Response> {
    success(status::Created, json!({}))
}
