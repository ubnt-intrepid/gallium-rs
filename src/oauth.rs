use std::borrow::{Borrow, Cow};
use std::str::FromStr;
use std::time::Duration;

use chrono::UTC;
use jsonwebtoken;
use uuid::Uuid;
use url::{Url, form_urlencoded};

use error::{AppResult, AppError};
use models::Scope;


#[derive(Debug, Deserialize)]
pub struct AuthorizationCode {
    pub user_id: i32,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<Vec<Scope>>,
}

impl AuthorizationCode {
    pub fn new(user_id: i32, client_id: &str, redirect_uri: &str, scope: Option<Vec<Scope>>) -> Self {
        AuthorizationCode {
            user_id,
            client_id: client_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scope: scope,
        }
    }

    pub fn encode(&self, secret: &[u8], lifetime: Duration) -> AppResult<String> {
        let header = Default::default();

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
        jsonwebtoken::encode(&header, &claims, secret).map_err(Into::into)
    }

    pub fn validate(token: &str, secret: &[u8]) -> AppResult<Self> {
        let validation = Default::default();
        jsonwebtoken::decode(token, secret, &validation)
            .map_err(Into::into)
            .map(|token_data| token_data.claims)
    }
}


pub enum ResponseType {
    Code,
    Token,
}

impl FromStr for ResponseType {
    type Err = ();
    fn from_str(s: &str) -> Result<ResponseType, ()> {
        match s {
            "code" => Ok(ResponseType::Code),
            "token" => Ok(ResponseType::Token),
            _ => Err(()),
        }
    }
}

pub struct AuthorizeRequestParam<'a> {
    pub response_type: ResponseType,
    pub client_id: Cow<'a, str>,
    pub redirect_uri: Option<Cow<'a, str>>,
    pub scope: Option<Vec<Scope>>,
    pub state: Option<Cow<'a, str>>,
}

impl<'a> AuthorizeRequestParam<'a> {
    pub fn from_url(url: &'a Url) -> AppResult<Self> {
        let (mut response_type, mut client_id, mut redirect_uri, mut scope, mut state) = (None, None, None, None, None);
        for (key, val) in url.query_pairs() {
            match &key as &str {
                "response_type" => response_type = Some(val),
                "client_id" => client_id = Some(val),
                "redirect_uri" => redirect_uri = Some(val),
                "scope" => scope = Some(val),
                "state" => state = Some(val),
                _ => (),
            }
        }
        let response_type: ResponseType = response_type
            .ok_or_else(|| AppError::from("invalid_request"))
            .and_then(|s| {
                s.parse().ok().ok_or_else(
                    || AppError::from("unsupported_response_type"),
                )
            })?;
        let client_id = client_id.ok_or_else(|| AppError::from("invalid_request"))?;
        let scope = scope.map(|s| s.split(" ").filter_map(|s| s.parse().ok()).collect());

        Ok(AuthorizeRequestParam {
            response_type,
            client_id,
            redirect_uri,
            scope,
            state,
        })
    }
}


pub enum GrantType<'a> {
    AuthorizationCode {
        code: Cow<'a, str>,
        client_id: Cow<'a, str>,
        client_secret: Cow<'a, str>,
        redirect_uri: Option<Cow<'a, str>>,
    },
    Password {
        username: Cow<'a, str>,
        password: Cow<'a, str>,
        client_id: Cow<'a, str>,
        client_secret: Cow<'a, str>,
        scope: Option<Vec<Scope>>,
    },
    ClientCredentials {
        client_id: Cow<'a, str>,
        client_secret: Cow<'a, str>,
        scope: Option<Vec<Scope>>,
    },
}

impl<'a> GrantType<'a> {
    pub fn from_vec(body: &'a [u8]) -> Result<Self, &'static str> {
        let (mut grant_type,
             mut code,
             mut username,
             mut password,
             mut client_id,
             mut scope,
             mut client_secret,
             mut redirect_uri) = (None, None, None, None, None, None, None, None);
        for (key, val) in form_urlencoded::parse(&body) {
            match &key as &str {
                "grant_type" => grant_type = Some(val),
                "code" => code = Some(val),
                "username" => username = Some(val),
                "password" => password = Some(val),
                "client_id" => client_id = Some(val),
                "client_secret" => client_secret = Some(val),
                "scope" => scope = Some(val.split(" ").filter_map(|s| s.parse().ok()).collect()),
                "redirect_uri" => redirect_uri = Some(val),
                _ => (),
            }
        }
        let client_id = client_id.ok_or("invalid_request")?;
        let client_secret = client_secret.ok_or("invalid_request")?;

        match grant_type.as_ref().map(|s| s.borrow()) {
            Some("authorization_code") => {
                let code = code.ok_or("invalid_request")?;
                Ok(GrantType::AuthorizationCode {
                    code,
                    redirect_uri,
                    client_id,
                    client_secret,
                })
            }
            Some("password") => {
                let username = username.ok_or("invalid_request")?;
                let password = password.ok_or("invalid_request")?;
                Ok(GrantType::Password {
                    username,
                    password,
                    client_id,
                    client_secret,
                    scope,
                })
            }
            Some("client_credentials") => Ok(GrantType::ClientCredentials {
                client_id,
                client_secret,
                scope,
            }),
            Some(_) => Err("unsupported_grant"),
            None => Err("invalid_grant"),
        }
    }
}
