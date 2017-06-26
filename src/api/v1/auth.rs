use std::borrow::Borrow;
use std::io;
use std::time::Duration;
use iron::prelude::*;
use iron::status;
use iron::headers::ContentType;
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::url::form_urlencoded;
use iron_json_response::JsonResponse;
use app::App;
use error::AppError;

const SECS_PER_ONE_DAY: u64 = 60 * 60 * 24;

pub(super) fn generate_token(req: &mut Request) -> IronResult<Response> {
    match req.headers.get::<ContentType>() {
        Some(&ContentType(Mime(TopLevel::Application, SubLevel::WwwFormUrlEncoded, _))) => (),
        _ => {
            return Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "invalid_request",
                })),
            )))
        }
    }

    let mut body = Vec::new();
    io::copy(&mut req.body, &mut body).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let (mut username, mut password, mut scope) = (None, None, None);
    for (key, val) in form_urlencoded::parse(&body) {
        match key.borrow() as &str {
            "grant_type" => {
                match val.borrow() as &str {
                    "password" => (),
                    "code" |
                    "token" |
                    "client_credentials" => {
                        return Err(IronError::new(AppError::from("OAuth"), (
                            status::BadRequest,
                            JsonResponse::json(json!({
                                "error": "unsupported_grant",
                            })),
                        )))
                    }
                    _ => {
                        return Err(IronError::new(AppError::from("OAuth"), (
                            status::BadRequest,
                            JsonResponse::json(json!({
                                "error": "invalid_grant",
                            })),
                        )))
                    }
                }
            }
            "username" => username = Some(val),
            "password" => password = Some(val),
            "scope" => scope = Some(val),
            _ => (),
        }
    }
    let (username, password) = match (username, password) {
        (Some(u), Some(p)) => (u, p),
        _ => {
            return Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "invalid_request",
                })),
            )))
        }
    };
    let scope: Option<Vec<&str>> = scope.as_ref().map(|scope| scope.split(" ").collect());

    let app = req.extensions.get::<App>().unwrap();
    let user = app.authenticate(&username, &password)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| {
            IronError::new(AppError::from("OAuth"), (
                status::Unauthorized,
                JsonResponse::json(json!({
                    "error": "unauthorized_client",
                })),
            ))
        })?;

    let token = app.generate_jwt(
        &user,
        scope.as_ref().map(|s| s.as_slice()),
        Duration::from_secs(SECS_PER_ONE_DAY),
    ).map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with((
        status::Ok,
        JsonResponse::json(json!({
            "access_token": token,
            "token_type": "bearer",
            "expires_in": SECS_PER_ONE_DAY,
        })),
    )))
}
