use bodyparser::Struct;
use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;

use error::AppError;
use db::DB;
use models::User;


#[derive(Serialize)]
pub struct EncodableUser {
    id: i32,
    name: String,
    email_address: String,
    created_at: String,
}

impl From<User> for EncodableUser {
    fn from(val: User) -> Self {
        EncodableUser {
            id: val.id,
            name: val.name,
            email_address: val.email_address,
            created_at: val.created_at.format("%c").to_string(),
        }
    }
}


pub(super) fn get_users(req: &mut Request) -> IronResult<Response> {
    let db = req.extensions.get::<DB>().unwrap();
    let users: Vec<_> = User::load_users(db)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(EncodableUser::from)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(users))))
}

pub(super) fn get_user(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let db = req.extensions.get::<DB>().unwrap();
    let user: EncodableUser = User::find_by_id(db, id)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .ok_or_else(|| IronError::new(AppError::from(""), status::NotFound))?
        .into();

    Ok(Response::with((status::Ok, JsonResponse::json(user))))
}

pub(super) fn create_user(req: &mut Request) -> IronResult<Response> {
    #[derive(Clone, Deserialize)]
    struct Params {
        name: String,
        email_address: String,
        password: String,
        screen_name: Option<String>,
        is_admin: Option<bool>,
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(AppError::from(""), status::BadRequest))?;

    let db = req.extensions.get::<DB>().unwrap();
    let user = User::create(
        db,
        &params.name,
        &params.password,
        &params.email_address,
        params.screen_name.as_ref().map(|s| s.as_str()),
        params.is_admin.clone(),
    ).map_err(|err| IronError::new(err, status::InternalServerError))
        .map(EncodableUser::from)?;

    Ok(Response::with((status::Created, JsonResponse::json(user))))
}
