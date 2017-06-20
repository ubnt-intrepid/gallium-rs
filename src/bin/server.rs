extern crate gallium;
extern crate iron;
extern crate mount;
extern crate router;
extern crate urlencoded;
extern crate flate2;
extern crate iron_slog;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;

use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use iron::prelude::*;
use iron::status;
use iron::headers::{CacheControl, CacheDirective, ContentType, ContentEncoding, Encoding};
use iron::modifiers::Header;
use iron::mime::{Mime, TopLevel, SubLevel};
use router::Router;
use urlencoded::UrlEncodedQuery;
use flate2::read::GzDecoder;
use slog::{Drain, Logger};
use iron_slog::{LoggerMiddleware, DefaultLogFormatter};

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

fn repo_path(req: &Request) -> PathBuf {
    let route = req.extensions.get::<Router>().unwrap();
    let project = route.find("project").unwrap();
    Path::new("/data").join(project)
}

fn handle_textfile<P: AsRef<Path>>(path: P) -> IronResult<Response> {
    if !path.as_ref().is_file() {
        return Err(IronError::new(CustomError, status::NotFound));
    }

    let content = OpenOptions::new()
        .read(true)
        .open(&path)
        .and_then(|mut f| {
            let mut content = String::new();
            f.read_to_string(&mut content).map(|_| content)
        })
        .map_err(|err| {
            IronError::new(err, (
                status::InternalServerError,
                format!(
                    "failed to read content: {}",
                    path.as_ref().display()
                ),
            ))
        })?;
    Ok(Response::with((
        status::Ok,
        Header(CacheControl(vec![CacheDirective::NoCache])),
        Header(ContentType::plaintext()),
        content,
    )))
}

fn handle_binaryfile<P: AsRef<Path>>(path: P) -> IronResult<Response> {
    if !path.as_ref().is_file() {
        return Err(IronError::new(CustomError, status::NotFound));
    }

    let content = OpenOptions::new()
        .read(true)
        .open(&path)
        .and_then(|mut f| {
            let mut content = Vec::new();
            f.read_to_end(&mut content).map(|_| content)
        })
        .map_err(|err| {
            IronError::new(err, (
                status::InternalServerError,
                format!(
                    "failed to read content: {}",
                    path.as_ref().display()
                ),
            ))
        })?;
    Ok(Response::with((
        status::Ok,
        Header(CacheControl(vec![CacheDirective::NoCache])),
        Header(ContentType::plaintext()),
        content,
    )))
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
    let repo_path = repo_path(req);

    let refs = git_command(service, repo_path)?;
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
    let repo_path = repo_path(req);

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

    let mut child: Child = Command::new("/usr/bin/git")
        .args(&[service, "--stateless-rpc", repo_path.to_str().unwrap()])
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    io::copy(&mut body_reader, child.stdin.as_mut().unwrap()).unwrap();

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
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!());

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

    router.get(
        "/:project/HEAD",
        |req: &mut Request| handle_textfile(repo_path(req).join("HEAD")),
        "head",
    );
    router.get(
        "/:project/objects/info/alternates",
        |req: &mut Request| handle_textfile(repo_path(req).join("objects/info/alternates")),
        "alternates",
    );
    router.get(
        "/:project/objects/info/http-alternates",
        |req: &mut Request| handle_textfile(repo_path(req).join("objects/info/http-alternates")),
        "http-alternates",
    );
    router.get(
        "/:project/objects/info/packs",
        |req: &mut Request| handle_textfile(repo_path(req).join("objects/info/packs")),
        "info-packs",
    );
    router.get(
        "/:project/objects/info/:file",
        |req: &mut Request| {
            let route = req.extensions.get::<Router>().unwrap();
            let file = route.find("file").unwrap();
            handle_textfile(repo_path(req).join("objects/info").join(file))
        },
        "info-files",
    );
    router.get(
        "/:project/objects/:prefix/:suffix",
        |req: &mut Request| {
            let route = req.extensions.get::<Router>().unwrap();
            let prefix = route.find("prefix").unwrap();
            let suffix = route.find("suffix").unwrap();
            handle_binaryfile(repo_path(req).join("objects").join(prefix).join(suffix))
                .map(|mut res| {
                    res.set_mut(Header(ContentType(Mime(
                        TopLevel::Application,
                        SubLevel::Ext("x-git-loose-object".to_string()),
                        Vec::new(),
                    ))));
                    res
                })
        },
        "object",
    );
    router.get(
        "/:project/objects/pack/:file",
        |req: &mut Request| {
            let route = req.extensions.get::<Router>().unwrap();
            let file = route.find("file").unwrap();
            handle_binaryfile(repo_path(req).join("objects/pack").join(file)).map(|mut res| {
                if file.ends_with(".pack") {
                    res.set_mut(Header(ContentType(Mime(
                        TopLevel::Application,
                        SubLevel::Ext("x-git-packed-objects".to_string()),
                        Vec::new(),
                    ))));
                }
                if file.ends_with(".idx") {
                    res.set_mut(Header(ContentType(Mime(
                        TopLevel::Application,
                        SubLevel::Ext("x-git-packed-objects-toc".to_string()),
                        Vec::new(),
                    ))));
                }
                res
            })
        },
        "pack",
    );

    let handler = LoggerMiddleware::new(router, logger, DefaultLogFormatter);
    Iron::new(handler).http("0.0.0.0:3000").unwrap();
}
