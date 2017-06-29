use std::borrow::Borrow;
use std::io;
use std::error::Error;
use std::time::Duration;
use iron::prelude::*;
use iron::status;
use iron::headers::{Authorization, Basic, ContentType, Location};
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::modifiers::Header;
use url::{Url, form_urlencoded};
use iron_json_response::{JsonResponse, JsonResponseMiddleware};
use router::Router;
use error::{AppResult, AppError};
use uuid::Uuid;
use chrono::UTC;
use jsonwebtoken;
use super::WWWAuthenticate;
use db::DB;
use config::Config;

use models::{User, OAuthApp, AccessToken};


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

    let scope: Option<Vec<&str>> = scope.as_ref().map(|s| s.split(" ").collect());

    let db = req.extensions.get::<DB>().unwrap();
    let config = req.extensions.get::<Config>().unwrap();

    let user = User::authenticate(&db, username, password)
        .map_err(server_error)?
        .ok_or_else(|| unauthorized(""))?;

    let client_id = client_id.ok_or_else(|| bad_request("invalid_request"))?;
    let oauth_app = OAuthApp::find_by_client_id(db, client_id.borrow())
        .map_err(server_error)?
        .ok_or_else(|| unauthorized("unauthorized_client"))?;

    // redirect_uri のデフォルト値はどうすべきか？
    let redirect_uri = redirect_uri.unwrap_or(oauth_app.redirect_uri.into());

    match response_type.as_ref().map(|s| s.borrow() as &str) {
        Some("code") => {
            let claims = AuthorizationCodeClaims {
                user_id: user.id,
                client_id: client_id.to_string(),
                redirect_uri: redirect_uri.to_string(),
                scope: scope
                    .as_ref()
                    .map(|s| s.into_iter().map(|s| s.to_string()).collect())
                    .unwrap_or_default(),
            };
            let code = claims
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

        Some("token") => {
            let new_token = AccessToken::create(db, user.id, oauth_app.id).map_err(
                server_error,
            )?;

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
        Some(_) => Err(bad_request("unsupported_response_type")),
        None => Err(bad_request("invalid_request")),
    }
}

// Endpoint for Access Token Request
// * https://tools.ietf.org/html/rfc6749#section-4.1.3
// * https://tools.ietf.org/html/rfc6749#section-4.3.3
// * https://tools.ietf.org/html/rfc6749#section-4.4.3
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
        _ => return Err(bad_request("invalid_request")),
    }

    let mut body = Vec::new();
    io::copy(&mut req.body, &mut body).map_err(server_error)?;

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
    let _scope = scope;

    let db = req.extensions.get::<DB>().unwrap();
    let config = req.extensions.get::<Config>().unwrap();
    let oauth_app = OAuthApp::authenticate(&db, client_id, client_secret)
        .map_err(server_error)?
        .ok_or_else(|| unauthorized("unauthorized_client"))?;

    let user = match grant_type.as_ref().map(|s| s.borrow() as &str) {
        Some("authorization_code") => {
            let code = code.ok_or_else(|| bad_request("invalid_request"))?;
            let claims = AuthorizationCodeClaims::validate(code.borrow(), config.jwt_secret.as_bytes())
                .map_err(server_error)?;
            if let Some(redirect_uri) = redirect_uri {
                if claims.redirect_uri != redirect_uri {
                    return Err(unauthorized("invalid_request"));
                }
            }
            if claims.client_id != oauth_app.client_id {
                return Err(unauthorized("unauthorized_client"));
            }
            User::find_by_id(db, claims.user_id)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("access_denied"))?
        }
        Some("password") => {
            let (username, password) = match (username, password) {
                (Some(u), Some(p)) => (u, p),
                _ => return Err(bad_request("invalid_request")),
            };
            User::authenticate(&db, &username, &password)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("access_denied"))?
        }
        Some("client_credentials") => {
            User::find_by_id(db, oauth_app.user_id)
                .map_err(server_error)?
                .ok_or_else(|| unauthorized("access_denied"))?
        }
        Some(ref _s) => Err(bad_request("unsupported_grant"))?,
        None => Err(bad_request("invalid_grant"))?,
    };

    let new_token = AccessToken::create(db, user.id, oauth_app.id).map_err(
        server_error,
    )?;

    Ok(Response::with((
        status::Ok,
        JsonResponse::json(json!({
            "access_token": new_token.hash,
            "token_type": "bearer",
        })),
    )))
}



#[derive(Debug, Deserialize)]
pub struct AuthorizationCodeClaims {
    pub user_id: i32,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
}

impl AuthorizationCodeClaims {
    fn validate(token: &str, secret: &[u8]) -> AppResult<Self> {
        jsonwebtoken::decode(token, secret, &Default::default())
            .map_err(Into::into)
            .map(|token_data| token_data.claims)
    }

    fn encode(&self, secret: &[u8], lifetime: Duration) -> AppResult<String> {
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
            "user_id": self.user_id,
            "client_id": self.client_id,
            "redirect_uri": self.redirect_uri,
            "scope": self.scope,
        });
        jsonwebtoken::encode(&Default::default(), &claims, secret).map_err(Into::into)
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
