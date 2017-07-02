use bodyparser::Struct;
use iron::prelude::*;
use iron::status;

use error::AppError;
use db::DB;
use models::User;
use super::{response, error};


#[derive(Route)]
#[get(path = "/users", handler = "get_users")]
pub(super) struct GetUsers;

fn get_users(req: &mut Request) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let users: Vec<_> = User::load_users(db)
        .map_err(error::server_error)?
        .into_iter()
        .map(EncodableUser::from)
        .collect();

    response::ok(users)
}



#[derive(Route)]
#[get(path = "/users/:id", handler = "get_user")]
pub(super) struct GetUser;

fn get_user(req: &mut Request, id: i32) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let user: EncodableUser = User::find_by_id(db, id)
        .map_err(error::server_error)?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?
        .into();

    response::ok(user)
}



#[derive(Route)]
#[post(path = "/users", handler = "create_user")]
pub(super) struct CreateUser;

fn create_user(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        name: String,
        password: String,
        screen_name: Option<String>,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| error::bad_request(""))?;

    let db = req.extensions.get::<DB>().unwrap();
    let user = User::create(
        db,
        &params.name,
        &params.password,
        params.screen_name.as_ref().map(|s| s.as_str()),
    ).map_err(error::server_error)
        .map(EncodableUser::from)?;

    response::created(user)
}



#[derive(Serialize)]
pub struct EncodableUser {
    id: i32,
    name: String,
    created_at: String,
}

impl From<User> for EncodableUser {
    fn from(val: User) -> Self {
        EncodableUser {
            id: val.id,
            name: val.name,
            created_at: val.created_at.format("%c").to_string(),
        }
    }
}
