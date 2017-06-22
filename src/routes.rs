use std::env;
use std::error;
use std::fmt;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use iron::prelude::*;
use iron::status;
use iron::headers::{CacheControl, CacheDirective, Encoding, ContentEncoding, ContentType};
use iron::mime::{Mime, TopLevel, SubLevel};
use iron::modifiers::Header;
use mount::Mount;
use router::Router;
use bodyparser::Struct;
use urlencoded::UrlEncodedQuery;
use flate2::read::GzDecoder;
use serde_json;

#[derive(Debug)]
struct CustomError;

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "custom error")
    }
}

impl error::Error for CustomError {
    fn description(&self) -> &str {
        "custom error"
    }
}


fn repo_route(path: &str) -> String {
    format!("/:user/:repository{}", path)
}

fn get_repository_path(req: &Request) -> IronResult<PathBuf> {
    let route = req.extensions.get::<Router>().unwrap();
    let user = route.find("user").unwrap();
    let repository = route.find("repository").unwrap();
    let repo_path = Path::new("/data").join(user).join(repository);
    if !repo_path.is_dir() {
        return Err(IronError::new(CustomError, status::NotFound));
    }
    Ok(repo_path)
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
    let repo_path = get_repository_path(req)?;

    let mut body = packet_write(&format!("# service=git-{}\n", service));
    body.extend(b"0000");
    let refs = Command::new("/usr/bin/git")
        .args(&[service, "--stateless-rpc", "--advertise-refs", "."])
        .current_dir(&repo_path)
        .output()
        .map_err(|err| IronError::new(err, status::InternalServerError))
        .and_then(|output| if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(IronError::new(CustomError, status::InternalServerError))
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
    let repo_path = get_repository_path(req)?;

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
                GzDecoder::new(&mut req.body)
                    .map_err(|err| IronError::new(err, status::InternalServerError))
                    .map(Box::new)?
            } else {
                Box::new(&mut req.body)
            }
        }
        _ => Box::new(&mut req.body),
    };

    let body = Command::new("/usr/bin/git")
        .args(&[service, "--stateless-rpc", repo_path.to_str().unwrap()])
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|mut child| {
            io::copy(&mut body_reader, child.stdin.as_mut().unwrap()).map(|_| child)
        })
        .and_then(|child| child.wait_with_output())
        .map_err(|err| IronError::new(err, status::InternalServerError))
        .and_then(|output| if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(IronError::new(CustomError, status::InternalServerError))
        })?;

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



#[derive(Serialize)]
struct SerdePublicKey {
    id: i32,
    created_at: String,
    title: String,
    key: String,
}

impl From<super::models::PublicKey> for SerdePublicKey {
    fn from(val: super::models::PublicKey) -> SerdePublicKey {
        SerdePublicKey {
            id: val.id,
            created_at: val.created_at.format("%c").to_string(),
            title: val.title,
            key: val.key,
        }
    }
}

fn handle_get_ssh_keys(_req: &mut Request) -> IronResult<Response> {
    use diesel::prelude::*;
    use diesel::pg::PgConnection;
    use super::models::PublicKey;
    use super::schema::public_keys;

    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    let keys: Vec<SerdePublicKey> = public_keys::table
        .load::<PublicKey>(&conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((
        status::Ok,
        Header(ContentType::json()),
        serde_json::to_string_pretty(&keys).unwrap(),
    )))
}

fn handle_add_ssh_key(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        user_id: i32,
        title: String,
        key: String,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| {
            IronError::new(CustomError, (
                status::BadRequest,
                Header(ContentType::json()),
                serde_json::to_string_pretty(&json!({
                    "message": "Failed to parse parameter as JSON",
                })).unwrap(),
            ))
        })?;

    use diesel::insert;
    use diesel::prelude::*;
    use diesel::pg::PgConnection;
    use super::schema::public_keys;
    use super::models::{PublicKey, NewPublicKey};

    let new_key = NewPublicKey {
        user_id: params.user_id,
        title: &params.title,
        key: &params.key,
    };
    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    let inserted_key: SerdePublicKey = insert(&new_key)
        .into(public_keys::table)
        .get_result::<PublicKey>(&conn)
        .map(Into::into)
        .map_err(|err| {
            let err_message = format!("Failed to insert requested key: {}", err);
            IronError::new(err, (
                status::InternalServerError,
                Header(ContentType::json()),
                serde_json::to_string_pretty(&json!({
                    "message": err_message,
                })).unwrap(),
            ))
        })?;

    Ok(Response::with((
        status::Created,
        Header(ContentType::json()),
        serde_json::to_string_pretty(&inserted_key).unwrap(),
    )))
}

pub fn create_handler() -> Mount {
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

    let mut api_router = Router::new();
    api_router.get("/user/keys", handle_get_ssh_keys, "get_ssh_keys");
    api_router.post("/user/keys", handle_add_ssh_key, "add_ssh_key");

    let mut mount = Mount::new();
    mount.mount("/", router);
    mount.mount("/api/v1", api_router);

    mount
}
