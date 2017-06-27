use iron::prelude::*;
use iron::headers::{Authorization, Bearer};
use iron::BeforeMiddleware;
use iron::status;
use error::AppError;
use app::App;

use diesel::prelude::*;
use models::User;
use schema::users;

pub struct AuthMiddleware;

impl BeforeMiddleware for AuthMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        let token = match req.headers.get::<Authorization<Bearer>>() {
            Some(&Authorization(Bearer { ref token })) => token,
            _ => {
                return Err(IronError::new(
                    AppError::from("Authorization"),
                    status::Unauthorized,
                ))
            }
        };

        let user = {
            let app: &App = req.extensions.get::<App>().unwrap();
            let conn = app.get_db_conn().map_err(|err| {
                IronError::new(err, status::InternalServerError)
            })?;

            let claims = app.validate_jwt(token).map_err(|err| {
                IronError::new(err, status::Unauthorized)
            })?;
            users::table
                .filter(users::dsl::id.eq(claims.user_id))
                .get_result::<User>(&*conn)
                .optional()
                .map_err(|err| IronError::new(err, status::InternalServerError))?
                .ok_or_else(|| IronError::new(AppError::from(""), status::Unauthorized))?
        };

        req.extensions.insert::<User>(user);

        Ok(())
    }
}
