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

use diesel::prelude::*;
use models::OAuthApp;
use schema::oauth_apps;

const SECS_PER_ONE_DAY: u64 = 60 * 60 * 24;


// See https://tools.ietf.org/html/rfc6749#section-4.3
pub(super) fn token_endpoint(req: &mut Request) -> IronResult<Response> {
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

    let (mut username, mut password, mut scope, mut client_id) = (None, None, None, None);
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
            "client_id" => client_id = Some(val),
            _ => (),
        }
    }
    let (username, password, client_id) = match (username, password, client_id) {
        (Some(u), Some(p), Some(c)) => (u, p, c),
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
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let oauth_app = oauth_apps::table
        .filter(oauth_apps::dsl::client_id.eq(client_id.borrow() as &str))
        .get_result::<OAuthApp>(&*conn)
        .optional()
        .map_err(|err| IronError::new(err, status::InternalServerError))?;
    if oauth_app.is_none() {
        return Err(IronError::new(AppError::from("OAuth"), (
            status::Unauthorized,
            JsonResponse::json(json!({
                "error": "unauthorized_client",
            })),
        )));
    }

    let user = app.authenticate(&username, &password)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| {
            IronError::new(AppError::from("OAuth"), (
                status::Unauthorized,
                JsonResponse::json(json!({
                    "error": "access_denied",
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
