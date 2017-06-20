extern crate gallium;
extern crate iron;
extern crate mount;
extern crate router;
extern crate urlencoded;

use std::path::Path;
use std::process::Command;
use iron::prelude::*;
use iron::status;
use iron::headers::{CacheControl, CacheDirective, ContentType};
use iron::modifiers::Header;
use iron::mime::{Mime, TopLevel, SubLevel};
use router::Router;
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

fn get_service_name(req: &mut Request) -> IronResult<&'static str> {
    let s = req.get_ref::<UrlEncodedQuery>()
        .ok()
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

fn git_command<P: AsRef<Path>>(service: &str, wd: P) -> IronResult<String> {
    Command::new("/usr/bin/git")
        .args(&[service, "--stateless-rpc", "--advertise-refs", "."])
        .current_dir(wd)
        .output()
        .map_err(|err| {
            let message = format!("failed to exec git: {}", err.to_string());
            IronError::new(err, (status::InternalServerError, message))
        })
        .and_then(|output| if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(IronError::new(CustomError, (
                status::InternalServerError,
                format!(
                    "git failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )))
        })
}

fn packet_write(data: &str) -> String {
    let mut s = format!("{:x}", data.len() + 4);
    if s.len() % 4 != 0 {
        let mut b = String::new();
        for _ in 0..(4 - s.len() % 4) {
            b.push('0');
        }
        s = b + &s;
    }
    s += data;
    s
}

fn handler(req: &mut Request) -> IronResult<Response> {
    let service = get_service_name(req)?;

    let route = req.extensions.get::<Router>().unwrap();
    let project = route.find("project").unwrap();

    let refs = git_command(service, Path::new("/data").join(project))?;
    let mut body = packet_write(&format!("# service=git-{}\n", service));
    body += "0000";
    body += &refs;

    let mut response = Response::new();
    response.set_mut(status::Ok);
    response.set_mut(Header(ContentType(Mime(
        TopLevel::Application,
        SubLevel::Ext(format!("x-git-{}-advertisement", service)),
        Vec::new(),
    ))));
    response.set_mut(Header(CacheControl(vec![CacheDirective::NoCache])));
    response.set_mut(body);

    Ok(response)
}

fn main() {
    let mut router = Router::new();
    router.get("/:project/info/refs", handler, "info_refs");

    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
