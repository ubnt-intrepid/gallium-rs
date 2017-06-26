use iron::prelude::*;
use iron::status;
use iron::headers::ContentType;
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::url::form_urlencoded;
use iron_json_response::JsonResponse;
use super::ApiError;
use std::borrow::Borrow;
use std::io;

pub(super) fn generate_token(req: &mut Request) -> IronResult<Response> {
    match req.headers.get::<ContentType>() {
        Some(&ContentType(Mime(TopLevel::Application, SubLevel::WwwFormUrlEncoded, _))) => (),
        _ => return Err(IronError::new(ApiError(""), status::BadRequest)),
    }

    let mut body = Vec::new();
    io::copy(&mut req.body, &mut body).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let (mut username, mut password, mut grant_type) = (None, None, None);
    for (key, val) in form_urlencoded::parse(&body) {
        match key.borrow() as &str {
            "username" => username = Some(val),
            "password" => password = Some(val),
            "grant_type" => grant_type = Some(val),
            _ => (),
        }
    }

    // TODO: authentication
    // TODO: generate JWT

    let token = String::new();

    Ok(Response::with((
        status::Ok,
        JsonResponse::json(json!({
            "access_token": token,
            "token_type": "bearer",
            "expires_in": 3600,
        })),
    )))
}
