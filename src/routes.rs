use std::io::Read;
use iron::prelude::*;
use iron::status;
use iron::headers::{Authorization, Basic, CacheControl, CacheDirective, Encoding, ContentEncoding,
                    ContentType};
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::modifiers::Header;
use mount::Mount;
use router::Router;
use urlencoded::UrlEncodedQuery;
use bcrypt;
use flate2::read::GzDecoder;
use diesel::prelude::*;

use api;
use app::{App, AppMiddleware, AppError};
use git::Repository;
use models::User;
use schema::users;


header! {
    (WWWAuthenticate, "WWW-Authenticate") => [String]
}


pub fn create_handler(app: App) -> Chain {
    let mut mount = Mount::new();
    mount.mount("/", create_git_handler());
    mount.mount("/api/v1", api::v1::create_api_handler());

    let mut chain = Chain::new(mount);
    chain.link_before(AppMiddleware::new(app));

    chain
}


fn create_git_handler() -> Router {
    let mut router = Router::new();
    router.get(repo_route("/info/refs"), handle_info_refs, "info_refs");
    router.post(
        repo_route("/git-receive-pack"),
        |req: &mut Request| handle_service_rpc(req, "receive-pack"),
        "receive-pack",
    );
    router.post(
        repo_route("/git-upload-pack"),
        |req: &mut Request| handle_service_rpc(req, "upload-pack"),
        "upload-pack",
    );

    router
}

fn basic_auth(req: &mut Request) -> IronResult<()> {
    match req.headers.get::<Authorization<Basic>>() {
        Some(&Authorization(Basic {
                                ref username,
                                password: Some(ref password),
                            })) => {
            let app = req.extensions.get::<App>().unwrap();
            let conn = app.get_db_conn().unwrap();
            let user: User = users::table
                .filter(users::dsl::name.eq(username))
                .get_result(&*conn)
                .unwrap();
            if !bcrypt::verify(password, &user.bcrypt_hash).unwrap_or(false) {
                return Err(IronError::new(AppError::Other(""), status::Unauthorized));
            }
        }
        Some(&Authorization(Basic {
                                username: _,
                                password: None,
                            })) => {
            return Err(IronError::new(AppError::Other(""), status::Unauthorized));
        }
        None => {
            return Err(IronError::new(AppError::Other(""), (
                status::Unauthorized,
                Header(WWWAuthenticate(
                    "Basic realm=\"main\"".to_owned(),
                )),
            )));
        }
    }
    Ok(())
}

fn repo_route(path: &str) -> String {
    format!("/:user/:project{}", path)
}

fn get_repository(req: &mut Request) -> IronResult<Repository> {
    basic_auth(req)?;

    let route = req.extensions.get::<Router>().unwrap();
    let user = route.find("user").unwrap();
    let project = route.find("project").unwrap();
    let project = project.trim_right_matches("/");
    if !project.ends_with(".git") {
        return Err(IronError::new(
            AppError::Other(
                "The repository URL should be end with '.git'",
            ),
            status::NotFound,
        ));
    }
    let project = project.trim_right_matches(".git");

    let app = req.extensions.get::<App>().unwrap();
    app.get_repository(user, project).map_err(|err| {
        IronError::new(err, status::NotFound)
    })
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
            Err(IronError::new(AppError::Other(""), (
                status::Forbidden,
                format!("Invalid service name: {}", s),
            )))
        }
        None => {
            Err(IronError::new(AppError::Other(""), (
                status::Forbidden,
                "Requires service name",
            )))
        }
    }
}

fn packet_write(data: &str) -> Vec<u8> {
    let s = format!("{:x}", data.len() + 4);
    if s.len() % 4 == 0 {
        return (s + data).as_bytes().into();
    }

    let mut ret = Vec::new();
    for _ in 0..(4 - s.len() % 4) {
        ret.push(b'0');
    }
    ret.extend(s.as_bytes());
    ret.extend(data.as_bytes());
    ret
}

fn handle_info_refs(req: &mut Request) -> IronResult<Response> {
    let service = get_service_name(req)?;
    let repo = get_repository(req)?;

    let mut body = packet_write(&format!("# service=git-{}\n", service));
    body.extend(b"0000");
    let refs = repo.run_rpc_command(service, None).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    body.extend(refs);

    Ok(Response::with((
        status::Ok,
        Header(CacheControl(vec![CacheDirective::NoCache])),
        Header(ContentType(Mime(
            TopLevel::Application,
            SubLevel::Ext(format!("x-git-{}-advertisement", service)),
            Vec::new(),
        ))),
        body,
    )))
}

fn handle_service_rpc(req: &mut Request, service: &str) -> IronResult<Response> {
    let repo = get_repository(req)?;

    match req.headers.get::<ContentType>() {
        Some(&ContentType(Mime(TopLevel::Application, SubLevel::Ext(ref s), _)))
            if format!("x-git-{}-request", service) == s.as_str() => (),
        _ => return Err(IronError::new(AppError::Other(""), status::Unauthorized)),
    }

    let mut body_reader: Box<Read> = match req.headers.get::<ContentEncoding>() {
        Some(&ContentEncoding(ref enc)) => {
            if enc.iter()
                .find(|&e| if let &Encoding::Gzip = e { true } else { false })
                .is_some()
            {
                GzDecoder::new(&mut req.body)
                    .map_err(|err| IronError::new(err, status::InternalServerError))
                    .map(Box::new)?
            } else {
                Box::new(&mut req.body)
            }
        }
        _ => Box::new(&mut req.body),
    };

    let body = repo.run_rpc_command(service, Some(&mut body_reader))
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    Ok(Response::with((
        status::Ok,
        Header(ContentType(Mime(
            TopLevel::Application,
            SubLevel::Ext(format!("x-git-{}-result", service)),
            Vec::new(),
        ))),
        body,
    )))
}
