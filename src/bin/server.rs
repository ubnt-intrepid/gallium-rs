extern crate gallium;
extern crate iron;
extern crate mount;
extern crate router;
extern crate urlencoded;
extern crate flate2;

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use iron::prelude::*;
use iron::status;
use iron::headers::{CacheControl, CacheDirective, ContentType, ContentEncoding, Encoding};
use iron::modifiers::Header;
use iron::mime::{Mime, TopLevel, SubLevel};
use router::Router;
use urlencoded::UrlEncodedQuery;
use flate2::read::GzDecoder;

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

fn handle_info_refs(req: &mut Request) -> IronResult<Response> {
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

fn handle_service_rpc(req: &mut Request, service: &str) -> IronResult<Response> {
    let route = req.extensions.get::<Router>().unwrap();
    let project = route.find("project").unwrap();

    match req.headers.get::<ContentType>() {
        Some(&ContentType(Mime(TopLevel::Application, SubLevel::Ext(ref s), _)))
            if format!("x-git-{}-request", service) == s.as_str() => (),
        _ => return Err(IronError::new(CustomError, status::Unauthorized)),
    }

    let mut body_reader: Box<Read> = match req.headers.get::<ContentEncoding>() {
        Some(&ContentEncoding(ref enc)) => {
            if enc.iter()
                .find(|&e| if let &Encoding::Gzip = e { true } else { false })
                .is_some()
            {
                let read = GzDecoder::new(&mut req.body).unwrap();
                Box::new(read)
            } else {
                Box::new(&mut req.body)
            }
        }
        _ => Box::new(&mut req.body),
    };

    let mut req_body = String::new();
    body_reader.read_to_string(&mut req_body).unwrap();

    let repo_path = Path::new("/data").join(project);
    let mut child: Child = Command::new("/usr/bin/git")
        .args(&[service, "--stateless-rpc", repo_path.to_str().unwrap()])
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(req_body.as_bytes())
        .unwrap();
    let body = child
        .wait_with_output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap();

    let mut response = Response::new();
    response.set_mut(Header(ContentType(Mime(
        TopLevel::Application,
        SubLevel::Ext(format!("x-git-{}-result", service)),
        Vec::new(),
    ))));
    response.set_mut(body);
    Ok(response)
}

fn main() {
    let mut router = Router::new();
    router.get("/:project/info/refs", handle_info_refs, "info_refs");
    router.post(
        "/:project/git-upload-pack",
        |req: &mut Request| handle_service_rpc(req, "upload-pack"),
        "upload-pack",
    );
    router.post(
        "/:project/git-receive-pack",
        |req: &mut Request| handle_service_rpc(req, "receive-pack"),
        "receive-pack",
    );
    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
