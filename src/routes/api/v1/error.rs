use iron::IronError;
use iron::headers::ContentType;
use iron::modifiers::Header;
use iron::status;
use serde_json;
use std::error::Error;
use error::AppError;

pub(super) fn not_found(message: &str) -> IronError {
    let body = serde_json::to_string(&json!({
        "error": "not_found",
        "error_description": message,
    })).unwrap_or("{}".into());
    IronError::new(AppError::from(message), (
        status::NotFound,
        Header(ContentType::json()),
        body,
    ))

}

pub(super) fn bad_request(message: &str) -> IronError {
    let body = serde_json::to_string(&json!({
        "error": "bad_request",
        "error_description": message,
    })).unwrap_or("{}".into());
    IronError::new(AppError::from(message), (
        status::NotFound,
        Header(ContentType::json()),
        body,
    ))
}

pub(super) fn server_error<E: 'static + Error + Send>(err: E) -> IronError {
    let message = err.to_string();
    let body = serde_json::to_string(&json!({
        "error": "server_error",
        "error_description": message,
    })).unwrap_or("{}".into());
    IronError::new(AppError::from(message), (
        status::NotFound,
        Header(ContentType::json()),
        body,
    ))
}
