extern crate gallium;
extern crate iron;
extern crate mount;
extern crate regex;
extern crate urlencoded;

use iron::prelude::*;
use iron::status;
use iron::headers::{CacheControl, CacheDirective};
use iron::modifiers::Header;
use regex::Regex;
use urlencoded::UrlEncodedQuery;

#[derive(Debug)]
struct CustomError;
impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "custom error")
    }
}
impl std::error::Error for CustomError {
    fn description(&self) -> &str {
        "custom error"
    }
}

fn get_service_name(req: &Request) -> IronResult<&'static str> {
    let s = req.extensions
        .get::<UrlEncodedQuery>()
        .and_then(|q| q.get("service").and_then(|s| s.into_iter().next()))
        .map(|s| s.as_str());
    match s {
        Some("git-receive-pack") => Ok("receive-pack"),
        Some("git-upload-pack") => Ok("upload-pack"),
        Some(ref s) => {
            Err(IronError::new(CustomError, (
                status::Forbidden,
                format!("Invalid service name: {}", s),
            )))
        }
        None => {
            Err(IronError::new(
                CustomError,
                (status::Forbidden, "Requires service name"),
            ))
        }
    }
}

fn handler(req: &mut Request) -> IronResult<Response> {
    let re = Regex::new(r"^(.*)/info/refs$").unwrap();
    if !re.is_match(&req.url.path().join("/")) {
        return Err(IronError::new(CustomError, (
            status::NotFound,
            format!("Not Found: {}", req.url),
        )));
    }
    let project = req.url.path().into_iter().next().unwrap();
    let service = get_service_name(req)?;

    Ok(Response::with((
        status::Ok,
        Header(CacheControl(vec![CacheDirective::NoCache])),
        format!("Ok: {}, {}", project, service),
    )))
}

fn main() {
    Iron::new(handler).http("0.0.0.0:3000").unwrap();
}
