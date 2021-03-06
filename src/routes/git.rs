use std::io::Read;
use iron::prelude::*;
use iron::status;
use iron::headers::{Authorization, Basic, CacheControl, CacheDirective, Encoding, ContentEncoding, ContentType};
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::modifiers::Header;
use router::Router;
use flate2::read::GzDecoder;
use error::AppError;
use models::{User, Project, Repository};
use super::WWWAuthenticate;
use db::DB;
use iron_router_ext::RegisterRoute;
use url::Url;
use std::borrow::Borrow;


pub(super) fn create_git_router() -> Router {
    let mut router = Router::new();
    router.register(InfoRefs);
    router.register(ReceivePack);
    router.register(UploadPack);
    router
}


fn check_repo_identifier<'a, 'b>(user: &'a str, project: &'b str) -> IronResult<(&'a str, &'b str)> {
    let project = project.trim_right_matches("/");
    if !project.ends_with(".git") {
        return Err(IronError::new(
            AppError::from(
                "The repository URL should be end with '.git'",
            ),
            status::NotFound,
        ));
    }
    let project = project.trim_right_matches(".git");

    Ok((user, project))
}

fn get_basic_auth_param<'a>(req: &'a Request) -> IronResult<(&'a str, &'a str)> {
    let &Authorization(Basic {
                           ref username,
                           ref password,
                       }) = req.headers.get::<Authorization<Basic>>().ok_or_else(|| {
        IronError::new(AppError::from(""), (
            status::Unauthorized,
            Header(WWWAuthenticate(
                "Basic realm=\"main\"".to_owned(),
            )),
        ))
    })?;

    let password = password.as_ref().ok_or_else(|| {
        IronError::new(AppError::from("Password is empty"), status::Unauthorized)
    })?;

    Ok((username, password))
}

fn open_repository(req: &mut Request, user: &str, project: &str) -> IronResult<(Project, Repository)> {
    let conn = DB::from_req(req).unwrap();
    let (user, project) = check_repo_identifier(user, project)?;
    let project = Project::find_by_id(&conn, (user, project))
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| IronError::new(AppError::from("Git"), status::NotFound))?;
    let repo = project.open_repository(&*conn).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    Ok((project, repo))

}

// TODO: check scope
// MEMO:
// * receive-pack
//   - public/private ともに認証必須
//* upload-pack
//   - private の場合のみ認証必須
//   - 現状は実質 public のみであるため認証回りは省略している
fn check_scope(req: &mut Request, service: &str, project: &Project) -> IronResult<()> {
    let conn = DB::from_req(req).unwrap();
    match service {
        "receive-pack" => {
            let (username, password) = get_basic_auth_param(req)?;
            let auth_user = User::authenticate(&conn, username, password)
                .map_err(|err| IronError::new(err, status::InternalServerError))?
                .ok_or_else(|| IronError::new(AppError::from(""), status::Unauthorized))?;

            if project.user_id != auth_user.id {
                return Err(IronError::new(AppError::from(""), status::Unauthorized));
            }
        }
        "upload-pack" => (),
        _ => unreachable!(),
    }
    Ok(())
}


fn get_service_name(req: &mut Request) -> IronResult<&'static str> {
    let url: Url = req.url.clone().into();
    let mut service = None;
    for (key, val) in url.query_pairs() {
        match key.borrow() {
            "service" => service = Some(val),
            _ => (),
        }
    }
    match service.as_ref().map(|s| s.borrow()) {
        Some("git-receive-pack") => Ok("receive-pack"),
        Some("git-upload-pack") => Ok("upload-pack"),
        Some(ref s) => {
            Err(IronError::new(AppError::from(""), (
                status::Forbidden,
                format!("Invalid service name: {}", s),
            )))
        }
        None => {
            Err(IronError::new(AppError::from(""), (
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


fn handle_service_rpc(req: &mut Request, user: &str, project: &str, service: &str) -> IronResult<Response> {
    let (project, repo) = open_repository(req, user, project)?;
    check_scope(req, service, &project)?;

    match req.headers.get::<ContentType>() {
        Some(&ContentType(Mime(TopLevel::Application, SubLevel::Ext(ref s), _)))
            if format!("x-git-{}-request", service) == s.as_str() => (),
        _ => return Err(IronError::new(AppError::from(""), status::Unauthorized)),
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



#[derive(Route)]
#[get(path = "/:user/:project/info/refs", handler = "info_refs")]
struct InfoRefs;

fn info_refs(req: &mut Request, user: String, project: String) -> IronResult<Response> {
    let service = get_service_name(req)?;
    let (project, repo) = open_repository(req, &user, &project)?;
    check_scope(req, service, &project)?;

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



#[derive(Route)]
#[post(path = "/:user/:project/git-receive-pack", handler = "receive_pack")]
struct ReceivePack;

#[inline]
fn receive_pack(req: &mut Request, user: String, project: String) -> IronResult<Response> {
    handle_service_rpc(req, &user, &project, "receive-pack")
}



#[derive(Route)]
#[post(path = "/:user/:project/git-upload-pack", handler = "upload_pack")]
struct UploadPack;

#[inline]
fn upload_pack(req: &mut Request, user: String, project: String) -> IronResult<Response> {
    handle_service_rpc(req, &user, &project, "upload-pack")
}
