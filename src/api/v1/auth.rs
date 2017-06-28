use std::borrow::Borrow;
use std::io;
use std::time::Duration;
use iron::prelude::*;
use iron::status;
use iron::headers::{Authorization, Basic, ContentType, Location};
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::modifiers::Header;
use url::{Url, form_urlencoded};
use iron_json_response::JsonResponse;
use app::App;
use error::{AppResult, AppError};
use uuid::Uuid;
use chrono::UTC;
use jsonwebtoken;

use diesel::prelude::*;
use models::{User, OAuthApp};
use schema::oauth_apps;

const SECS_PER_ONE_DAY: u64 = 60 * 60 * 24;

header! {
    (WWWAuthenticate, "WWW-Authenticate") => [String]
}

// Endpoint for Authorization Request
// See https://tools.ietf.org/html/rfc6749#section-4.1.
pub(super) fn authorize_endpoint(req: &mut Request) -> IronResult<Response> {
    let (username, password) = match req.headers.get::<Authorization<Basic>>() {
        Some(&Authorization(Basic {
                                ref username,
                                password: Some(ref password),
                            })) => (username, password),
        _ => {
            return Err(IronError::new(AppError::from("OAuth"), (
                status::Unauthorized,
                Header(WWWAuthenticate(
                    "realm=\"Basic\"".to_owned(),
                )),
            )))
        }
    };

    let url: Url = req.url.clone().into();

    let (mut response_type, mut client_id, mut redirect_uri, mut scope, mut state) = (None, None, None, None, None);
    for (key, val) in url.query_pairs() {
        match key.borrow() as &str {
            "response_type" => response_type = Some(val),
            "client_id" => client_id = Some(val),
            "redirect_uri" => redirect_uri = Some(val),
            "scope" => scope = Some(val),
            "state" => state = Some(val),
            _ => (),
        }
    }

    match response_type.as_ref().map(|s| s.borrow() as &str) {
        Some("code") => {
            let client_id = client_id.ok_or_else(|| {
                IronError::new(AppError::from("OAuth"), (
                    status::BadRequest,
                    JsonResponse::json(json!({
                        "error": "invalid_request",
                        "error_description": "`client_id` is required",
                    })),
                ))
            })?;

            let app = req.extensions.get::<App>().unwrap();
            let conn = app.get_db_conn().map_err(|err| {
                IronError::new(err, (
                    status::InternalServerError,
                    JsonResponse::json(json!({
                        "error": "server_error",
                    })),
                ))
            })?;
            let oauth_app = oauth_apps::table
                .filter(oauth_apps::dsl::client_id.eq(client_id.borrow() as &str))
                .get_result::<OAuthApp>(&*conn)
                .optional()
                .map_err(|err| {
                    IronError::new(err, (
                        status::InternalServerError,
                        JsonResponse::json(json!({
                            "error": "server_error",
                        })),
                    ))
                })?
                .ok_or_else(|| {
                    IronError::new(AppError::from("OAuth"), (
                        status::Unauthorized,
                        JsonResponse::json(json!({
                            "error": "unauthorized_client",
                        })),
                    ))
                })?;

            let user = app.authenticate(username, password)
                .map_err(|err| {
                    IronError::new(err, (
                        status::InternalServerError,
                        JsonResponse::json(json!({
                            "error": "server_error",
                        })),
                    ))
                })?
                .ok_or_else(|| {
                    IronError::new(AppError::from("OAuth"), (status::Unauthorized))
                })?;

            // redirect_uri のデフォルト値はどうすべきか？
            let redirect_uri = redirect_uri.unwrap_or(oauth_app.redirect_uri.into());

            let scope: Option<Vec<&str>> = scope.as_ref().map(|s| s.split(" ").collect());
            let code = generate_authorization_code(
                &user,
                client_id.borrow(),
                redirect_uri.borrow(),
                scope.as_ref().map(|s| s.as_slice()),
                Duration::from_secs(10 * 60),
                app.config().jwt_secret.as_bytes(),
            ).map_err(|err| {
                IronError::new(err, (
                    status::InternalServerError,
                    JsonResponse::json(json!({
                        "error": "server_error",
                        "error_description": "Failed to generate authorization code",
                    })),
                ))
            })?;

            // Build redirect URL
            let mut location = Url::parse(redirect_uri.borrow()).unwrap();
            {
                let mut queries = location.query_pairs_mut();
                queries.append_pair("code", &code);
                if let Some(state) = state {
                    queries.append_pair("state", state.borrow());
                }
            }

            Ok(Response::with((
                status::Found,
                Header(Location(location.as_str().to_owned())),
            )))
        }
        Some(ref s) => {
            Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "unsupported_response_type",
                    "error_description": format!("`{}` is not a valid response_type", (s.borrow() as &str)),
                })),
            )))
        }
        None => {
            Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "invalid_request",
                    "error_description": "`response_type` is required",
                })),
            )))
        }
    }
}

// Endpoint for Access Token Request
// * https://tools.ietf.org/html/rfc6749#section-4.1
// * https://tools.ietf.org/html/rfc6749#section-4.3
// * https://tools.ietf.org/html/rfc6749#section-4.4
pub(super) fn token_endpoint(req: &mut Request) -> IronResult<Response> {
    let (client_id, client_secret) = match req.headers.get::<Authorization<Basic>>() {
        Some(&Authorization(Basic {
                                ref username,
                                password: Some(ref password),
                            })) => (username, password),
        _ => {
            return Err(IronError::new(AppError::from("OAuth"), (
                status::Unauthorized,
                Header(WWWAuthenticate(
                    "realm=\"Basic\"".to_owned(),
                )),
            )))
        }
    };

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

    let (mut grant_type, mut username, mut password, mut scope, mut code, mut redirect_uri) =
        (None, None, None, None, None, None);
    for (key, val) in form_urlencoded::parse(&body) {
        match key.borrow() as &str {
            "grant_type" => grant_type = Some(val),
            "username" => username = Some(val),
            "password" => password = Some(val),
            "scope" => scope = Some(val),
            "code" => code = Some(val),
            "redirect_uri" => redirect_uri = Some(val),
            _ => (),
        }
    }

    let scope: Option<Vec<&str>> = scope.as_ref().map(|scope| scope.split(" ").collect());

    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let oauth_app = app.authenticate_app(client_id, client_secret)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| {
            IronError::new(AppError::from("OAuth"), status::Unauthorized)
        })?;

    let token = match grant_type.as_ref().map(|s| s.borrow() as &str) {
        Some("authorization_code") => {
            let code = code.ok_or_else(|| {
                IronError::new(AppError::from("OAuth"), (
                    status::BadRequest,
                    JsonResponse::json(json!({
                        "error": "invalid_request",
                    })),
                ))
            })?;

            let claims: JWTClaims = validate_authorization_code(code.borrow(), app.config().jwt_secret.as_bytes())
                .map_err(|err| IronError::new(err, status::InternalServerError))?;

            if let Some(redirect_uri) = redirect_uri {
                if claims.redirect_uri != redirect_uri {
                    return Err(IronError::new(AppError::from("OAuth"), (
                        status::Unauthorized,
                        JsonResponse::json(json!({
                            "error": "invalid_request",
                        })),
                    )));
                }
            }

            if claims.client_id != oauth_app.client_id {
                return Err(IronError::new(AppError::from("OAuth"), (
                    status::Unauthorized,
                    JsonResponse::json(json!({
                        "error": "unauthorized_client",
                    })),
                )));
            }

            // TODO: generate access_token
            let token = String::new();
            token
        }
        Some("password") => {
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
            let _user = app.authenticate(&username, &password)
                .map_err(|err| IronError::new(err, status::InternalServerError))?
                .ok_or_else(|| {
                    IronError::new(AppError::from("OAuth"), (
                        status::Unauthorized,
                        JsonResponse::json(json!({
                            "error": "access_denied",
                        })),
                    ))
                })?;

            // TODO: generate access_token
            let _scope = scope;
            let token = "".to_owned();
            token
        }
        Some("client_credentials") => {
            // TODO: get user info related to client_id
            let _user = 0;

            // TODO: generate access_token
            let _scope = scope;
            let token = "".to_owned();
            token
        }
        Some(ref _s) => {
            Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "unsupported_grant",
                })),
            )))?
        }
        None => {
            Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "invalid_grant",
                })),
            )))?
        }
    };

    Ok(Response::with((
        status::Ok,
        JsonResponse::json(json!({
            "access_token": token,
            "token_type": "bearer",
            "expires_in": SECS_PER_ONE_DAY,
        })),
    )))
}



#[derive(Debug, Deserialize)]
pub struct JWTClaims {
    pub user_id: i32,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
}

fn generate_authorization_code(
    user: &User,
    client_id: &str,
    redirect_uri: &str,
    scope: Option<&[&str]>,
    lifetime: Duration,
    secret: &[u8],
) -> AppResult<String> {
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
        "exp": iat.timestamp() + lifetime.as_secs() as i64,
        "user_id": user.id,
        "client_id": client_id,
        "redirect_uri": redirect_uri,
        "scope": scope,
    });
    jsonwebtoken::encode(&Default::default(), &claims, secret).map_err(Into::into)
}

fn validate_authorization_code(token: &str, secret: &[u8]) -> AppResult<JWTClaims> {
    jsonwebtoken::decode(token, secret, &Default::default())
        .map_err(Into::into)
        .map(|token_data| token_data.claims)
}
