use bodyparser::Struct;
use bcrypt;
use diesel::insert;
use diesel::prelude::*;
use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;

use app::App;
use error::AppError;
use models::{User, NewUser};
use schema::users;


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
    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let users: Vec<EncodableUser> = users::table
        .load::<User>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Response::with((status::Ok, JsonResponse::json(users))))
}

pub(super) fn get_user(req: &mut Request) -> IronResult<Response> {
    let router = req.extensions.get::<Router>().unwrap();
    let id: i32 = router.find("id").and_then(|s| s.parse().ok()).unwrap();

    let app: &App = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let user: EncodableUser = users::table
        .filter(users::dsl::id.eq(id))
        .get_result::<User>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
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
    }
    let params = req.get::<Struct<Params>>()
        .ok()
        .and_then(|s| s)
        .ok_or_else(|| IronError::new(AppError::from(""), status::BadRequest))?;

    let bcrypt_hash = bcrypt::hash(&params.password, bcrypt::DEFAULT_COST)
        .map_err(|err| IronError::new(err, status::InternalServerError))?;

    let new_user = NewUser {
        name: &params.name,
        email_address: &params.email_address,
        bcrypt_hash: &bcrypt_hash,
        screen_name: params.screen_name.as_ref().map(|s| s.as_str()),
    };

    let app = req.extensions.get::<App>().unwrap();
    let conn = app.get_db_conn().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let inserted_user: EncodableUser = insert(&new_user)
        .into(users::table)
        .get_result::<User>(&*conn)
        .map_err(|err| IronError::new(err, status::InternalServerError))?
        .into();

    Ok(Response::with(
        (status::Created, JsonResponse::json(inserted_user)),
    ))
}
