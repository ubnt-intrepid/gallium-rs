use std::borrow::Borrow;
use std::error::Error;
use std::io;
use std::time::Duration;

use iron::prelude::*;
use iron::status;
use iron::headers::{Authorization, Basic, ContentType, Location};
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::modifiers::Header;
use iron_json_response::{JsonResponse, JsonResponseMiddleware};
use router::Router;
use url::Url;

use config::Config;
use db::DB;
use error::AppError;
use models::{User, OAuthApp, AccessToken};
use oauth::{AuthorizationCode, AuthorizeRequestParam, ResponseType, GrantType};

use super::WWWAuthenticate;


pub fn create_oauth_handler() -> Chain {
    let mut router = Router::new();
    router.get("/authorize", authorize_endpoint, "authorize");
    router.post("/token", token_endpoint, "token");

    let mut chain = Chain::new(router);
    chain.link_after(JsonResponseMiddleware::new());
    chain
}


// Endpoint for Authorization Request
// * https://tools.ietf.org/html/rfc6749#section-4.1.1
// * https://tools.ietf.org/html/rfc6749#section-4.2.1
pub(super) fn authorize_endpoint(req: &mut Request) -> IronResult<Response> {
    let url: Url = req.url.clone().into();
    let AuthorizeRequestParam {
        response_type,
        client_id,
        redirect_uri,
        scope,
        state,
    } = AuthorizeRequestParam::from_url(&url).map_err(|err| {
        bad_request(&err.to_string())
    })?;

    let (username, password) = get_credential(req)?;
    let db = req.extensions.get::<DB>().unwrap();
    let user = User::authenticate(&db, &username, &password)
        .map_err(server_error)?
        .ok_or_else(|| unauthorized(""))?;

    match response_type {
        // Authorization Code Flow
        ResponseType::Code => {
            let db = req.extensions.get::<DB>().unwrap();
            let oauth_app = OAuthApp::find_by_client_id(db, client_id.borrow())
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("unauthorized_client"))?;

            // redirect_uri のデフォルト値はどうすべきか？
            let redirect_uri = redirect_uri.unwrap_or(oauth_app.redirect_uri.into());

            let config = req.extensions.get::<Config>().unwrap();
            let code = AuthorizationCode::new(user.id, &client_id, &redirect_uri, scope)
                .encode(config.jwt_secret.as_bytes(), Duration::from_secs(10 * 60))
                .map_err(server_error)?;

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

        // Implicit Flow
        ResponseType::Token => {
            let db = req.extensions.get::<DB>().unwrap();
            let oauth_app = OAuthApp::find_by_client_id(db, client_id.borrow())
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("unauthorized_client"))?;

            // redirect_uri のデフォルト値はどうすべきか？
            let redirect_uri = redirect_uri.unwrap_or(oauth_app.redirect_uri.into());

            let new_token = AccessToken::create(db, user.id, oauth_app.id, scope)
                .map_err(server_error)?;

            // Build redirect URL
            let mut location = Url::parse(redirect_uri.borrow()).unwrap();
            {
                let mut queries = location.query_pairs_mut();
                queries.append_pair("access_token", &new_token.hash);
                queries.append_pair("token_type", "bearer");
                if let Some(state) = state {
                    queries.append_pair("state", state.borrow());
                }
            }

            Ok(Response::with((
                status::Found,
                Header(Location(location.as_str().to_owned())),
            )))
        }
    }
}

// Endpoint for Access Token Request
// * https://tools.ietf.org/html/rfc6749#section-4.1.3
// * https://tools.ietf.org/html/rfc6749#section-4.3.3
// * https://tools.ietf.org/html/rfc6749#section-4.4.3
pub(super) fn token_endpoint(req: &mut Request) -> IronResult<Response> {
    let mut body = Vec::new();
    let grant_type = match req.headers.get::<ContentType>() {
        Some(&ContentType(Mime(TopLevel::Application, SubLevel::WwwFormUrlEncoded, _))) => {
            io::copy(&mut req.body, &mut body).map_err(server_error)?;
            GrantType::from_vec(&body).map_err(|err| bad_request(&err))?
        }
        _ => return Err(bad_request("invalid_request")),
    };

    let (user, oauth_app, scope) = match grant_type {
        GrantType::AuthorizationCode {
            code,
            client_id,
            client_secret,
            redirect_uri,
        } => {
            let config = req.extensions.get::<Config>().unwrap();
            let secret = &config.jwt_secret;

            let code = AuthorizationCode::validate(&code, secret.as_bytes())
                .map_err(server_error)?;
            if let Some(redirect_uri) = redirect_uri {
                if code.redirect_uri != redirect_uri {
                    return Err(unauthorized("invalid_request"));
                }
            }
            if code.client_id != client_id {
                return Err(unauthorized("unauthorized_client"));
            }

            let db = req.extensions.get::<DB>().unwrap();
            let user = User::find_by_id(db, code.user_id)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("access_denied"))?;
            let oauth_app = OAuthApp::authenticate(&db, &client_id, &client_secret)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("unauthorized_client"))?;

            (user, oauth_app, code.scope)
        }

        GrantType::Password {
            username,
            password,
            client_id,
            client_secret,
            scope,
        } => {
            let db = req.extensions.get::<DB>().unwrap();
            let user = User::authenticate(&db, &username, &password)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("access_denied"))?;
            let oauth_app = OAuthApp::authenticate(&db, &client_id, &client_secret)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("unauthorized_client"))?;
            (user, oauth_app, scope)
        }

        GrantType::ClientCredentials {
            client_id,
            client_secret,
            scope,
        } => {
            let db = req.extensions.get::<DB>().unwrap();
            let oauth_app = OAuthApp::authenticate(&db, &client_id, &client_secret)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("unauthorized_client"))?;
            let user = User::find_by_id(db, oauth_app.user_id)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("access_denied"))?;
            (user, oauth_app, scope)
        }
    };

    let db = req.extensions.get::<DB>().unwrap();
    let new_token = AccessToken::create(db, user.id, oauth_app.id, scope)
        .map_err(server_error)?;

    Ok(Response::with((
        status::Ok,
        JsonResponse::json(json!({
            "access_token": new_token.hash,
            "token_type": "bearer",
        })),
    )))
}


fn get_credential(req: &mut Request) -> IronResult<(String, String)> {
    match req.headers.get::<Authorization<Basic>>() {
        Some(&Authorization(Basic {
                                ref username,
                                password: Some(ref password),
                            })) => Ok((username.clone(), password.clone())),
        _ => {
            return Err(IronError::new(AppError::from("OAuth"), (
                status::Unauthorized,
                Header(WWWAuthenticate(
                    "realm=\"Basic\"".to_owned(),
                )),
            )))
        }
    }
}

fn bad_request(oauth_error: &str) -> IronError {
    IronError::new(AppError::from("OAuth"), (
        status::BadRequest,
        JsonResponse::json(json!({
            "error": oauth_error,
        })),
    ))
}

fn unauthorized(oauth_error: &str) -> IronError {
    IronError::new(AppError::from("OAuth"), (
        status::Unauthorized,
        JsonResponse::json(json!({
            "error": oauth_error,
        })),
    ))
}

fn server_error<E: Error + Send + 'static>(err: E) -> IronError {
    IronError::new(err, (
        status::InternalServerError,
        JsonResponse::json(json!({
            "error": "server_error",
        })),
    ))
}
