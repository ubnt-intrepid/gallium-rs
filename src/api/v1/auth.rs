use std::borrow::Borrow;
use std::io;
use std::time::Duration;
use iron::prelude::*;
use iron::status;
use iron::headers::{ContentType, Location};
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::modifiers::Header;
use url::{Url, form_urlencoded};
use iron_json_response::JsonResponse;
use router::url_for;
use app::App;
use error::AppError;

use diesel::prelude::*;
use models::OAuthApp;
use schema::oauth_apps;

const SECS_PER_ONE_DAY: u64 = 60 * 60 * 24;

// Endpoint for Authorization Request
// See https://tools.ietf.org/html/rfc6749#section-4.1.
pub(super) fn authorize_endpoint(req: &mut Request) -> IronResult<Response> {
    let url: Url = req.url.clone().into();

    let (mut response_type, mut client_id, mut redirect_uri, mut scope, mut state) =
        (None, None, None, None, None);
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
        Some("code") => (),
        Some(ref s) => {
            return Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "unsupported_response_type",
                    "error_description": format!("`{}` is not a valid response_type", (s.borrow() as &str)),
                })),
            )))
        }
        None => {
            return Err(IronError::new(AppError::from("OAuth"), (
                status::BadRequest,
                JsonResponse::json(json!({
                    "error": "invalid_request",
                    "error_description": "`response_type` is required",
                })),
            )))
        }
    }

    let client_id = client_id.ok_or_else(|| {
        IronError::new(AppError::from("OAuth"), (
            status::BadRequest,
            JsonResponse::json(json!({
                "error": "invalid_request",
                "error_description": "`client_id` is required",
            })),
        ))
    })?;

    // TODO: check scope
    let _scope = scope;

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

    let redirect_uri = redirect_uri.unwrap_or(oauth_app.redirect_uri.into());
    let redirect_uri = if redirect_uri.borrow() as &str == "urn:ietf:wg:oauth:2.0:oob" {
        let url: Url = url_for(req, "auth/approval", Default::default()).into();
        url.as_str().to_owned().into()
    } else {
        redirect_uri
    };

    // TODO: generate authorization code
    let code = "xxxx".to_owned();

    // Build redirect URL
    let queries = {
        let mut queries = form_urlencoded::Serializer::new(String::new());
        queries.append_pair("code", &code);
        if let Some(state) = state {
            queries.append_pair("state", state.borrow());
        }
        queries.finish()
    };
    let mut location = Url::parse(redirect_uri.borrow()).unwrap();
    location.set_query(Some(queries.as_str()));

    Ok(Response::with((
        status::Found,
        Header(Location(location.as_str().to_owned())),
    )))
}

pub(super) fn approval_endpoint(req: &mut Request) -> IronResult<Response> {
    let url: Url = req.url.clone().into();

    let (mut code, mut state) = (None, None);
    for (key, val) in url.query_pairs() {
        match key.borrow() as &str {
            "code" => code = Some(val),
            "state" => state = Some(val),
            _ => (),
        }
    }

    Ok(Response::with((
        status::Ok,
        JsonResponse::json(json!({
            "code": code,
            "state": state,
        })),
    )))
}

// Endpoiint for Access Token Request
// See https://tools.ietf.org/html/rfc6749#section-4.1 and
//     https://tools.ietf.org/html/rfc6749#section-4.3.
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
