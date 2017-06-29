use std::borrow::Borrow;
use std::io::{self, Read};
use std::str::FromStr;
use std::time::Duration;

use chrono::UTC;
use jsonwebtoken;
use uuid::Uuid;
use url::{Url, form_urlencoded};

use error::{AppResult, AppError};


#[derive(Debug, Deserialize)]
pub struct AuthorizationCode {
    pub user_id: i32,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
}

impl AuthorizationCode {
    pub fn new(user_id: i32, client_id: &str, redirect_uri: &str) -> Self {
        AuthorizationCode {
            user_id,
            client_id: client_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scope: Vec::new(),
        }
    }

    pub fn scope<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scope.extend(scopes.into_iter().map(Into::into));
        self
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

pub struct AuthorizeRequestParam {
    pub response_type: ResponseType,
    pub client_id: String,
    pub redirect_uri: Option<String>,
    pub scope: Option<Vec<String>>,
    pub state: Option<String>,
}

impl AuthorizeRequestParam {
    pub fn from_url(url: &Url) -> AppResult<Self> {
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

        let response_type: ResponseType = response_type
            .ok_or_else(|| AppError::from("invalid_request"))
            .and_then(|s| {
                s.parse().ok().ok_or_else(
                    || AppError::from("unsupported_response_type"),
                )
            })?;
        let client_id = client_id.ok_or_else(|| AppError::from("invalid_request"))?;

        Ok(AuthorizeRequestParam {
            response_type,
            client_id: client_id.into_owned(),
            redirect_uri: redirect_uri.map(|s| s.into_owned()),
            scope: scope.map(|s| s.split(" ").map(|s| s.to_string()).collect()),
            state: state.map(|s| s.into_owned()),
        })
    }
}


pub enum GrantType {
    AuthorizationCode,
    Password,
    ClientCredentials,
}

impl FromStr for GrantType {
    type Err = ();
    fn from_str(s: &str) -> Result<GrantType, Self::Err> {
        match s {
            "authorization_code" => Ok(GrantType::AuthorizationCode),
            "password" => Ok(GrantType::Password),
            "client_credentials" => Ok(GrantType::ClientCredentials),
            _ => Err(()),
        }
    }
}

pub struct TokenEndpointParams {
    pub grant_type: GrantType,
    pub username: Option<String>,
    pub password: Option<String>,
    pub scope: Option<Vec<String>>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
}

pub fn parse_token_endpoint_params<R: Read>(read: &mut R) -> AppResult<TokenEndpointParams> {
    let mut body = Vec::new();
    io::copy(read, &mut body)?;

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

    let grant_type = grant_type
        .ok_or_else(|| AppError::from("invalid_grant"))?
        .parse()
        .map_err(|_| AppError::from("unsupported_grant"))?;

    Ok(TokenEndpointParams {
        grant_type,
        username: username.map(|s| s.into_owned()),
        password: password.map(|s| s.into_owned()),
        scope: scope.as_ref().map(|scope| {
            scope.split(" ").map(|s| s.to_string()).collect()
        }),
        code: code.map(|s| s.into_owned()),
        redirect_uri: redirect_uri.map(|s| s.into_owned()),
    })
}
