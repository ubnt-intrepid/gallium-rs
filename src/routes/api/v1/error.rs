use iron::IronError;
use iron::status;
use iron_json_response::JsonResponse;
use std::error::Error;
use error::AppError;


pub(super) fn bad_request(message: &str) -> IronError {
    IronError::new(AppError::from(message), (
        status::BadRequest,
        JsonResponse::json(json!({
            "error": "bad_request",
            "error_description": message,
        })),
    ))
}

pub(super) fn server_error<E: 'static + Error + Send>(err: E) -> IronError {
    let message = err.to_string();
    IronError::new(err, (
        status::InternalServerError,
        JsonResponse::json(json!({
            "error": "server_error",
            "error_description": message,
        })),
    ))
}
