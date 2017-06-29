use chrono::UTC;
use jsonwebtoken;
use std::time::Duration;
use uuid::Uuid;

use error::AppResult;


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
