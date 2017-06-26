use std::borrow::Borrow;
use std::io;
use chrono::UTC;
use jsonwebtoken;
use iron::prelude::*;
use iron::status;
use iron::headers::ContentType;
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::url::form_urlencoded;
use iron_json_response::JsonResponse;
use uuid::Uuid;
use app::App;
use super::ApiError;

pub(super) fn generate_token(req: &mut Request) -> IronResult<Response> {
    match req.headers.get::<ContentType>() {
        Some(&ContentType(Mime(TopLevel::Application, SubLevel::WwwFormUrlEncoded, _))) => (),
        _ => {
            return Err(IronError::new(ApiError("OAuth"), (
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

    let (mut username, mut password, mut _scope) = (None, None, None);
    for (key, val) in form_urlencoded::parse(&body) {
        match key.borrow() as &str {
            "grant_type" => {
                match val.borrow() as &str {
                    "password" => (),
                    "code" |
                    "token" |
                    "client_credentials" => {
                        return Err(IronError::new(ApiError("OAuth"), (
                            status::BadRequest,
                            JsonResponse::json(json!({
                                "error": "unsupported_grant",
                            })),
                        )))
                    }
                    _ => {
                        return Err(IronError::new(ApiError("OAuth"), (
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
            "scope" => _scope = Some(val),
            _ => (),
        }
    }
    let (username, password) = match (username, password) {
        (Some(u), Some(p)) => (u, p),
        _ => {
            return Err(IronError::new(ApiError("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "invalid_request",
                })),
            )))
        }
    };
    let scope: Option<Vec<&str>> = _scope.as_ref().map(|scope| scope.split(" ").collect());

    let app = req.extensions.get::<App>().unwrap();
    let user = app.authenticate(&username, &password)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| {
            IronError::new(ApiError("OAuth"), (
                status::Unauthorized,
                JsonResponse::json(json!({
                    "error": "unauthorized_client",
                })),
            ))
        })?;

    // TODO: move secret key to Config
    let secret = "secret-key";
    let expires_in = 3600;
    let iss = "http://localhost:3000/";
    let aud = vec!["http://localhost:3000/"];

    let jti = Uuid::new_v4();
    let iat = UTC::now();
    let claims = json!({
        "jti": jti.to_string(),
        "iss": iss,
        "aud": aud,
        "sub": "access_token",
        "iat": iat.timestamp(),
        "nbf": iat.timestamp(),
        "exp": iat.timestamp() + expires_in,
        "priv": {
            "user_id": user.id,
            "username": user.name,
            "scope": scope,
        }
    });
    let token = jsonwebtoken::encode(&Default::default(), &claims, secret.as_bytes())
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with((
        status::Ok,
        JsonResponse::json(json!({
            "access_token": token,
            "token_type": "bearer",
            "expires_in": expires_in,
        })),
    )))
}
