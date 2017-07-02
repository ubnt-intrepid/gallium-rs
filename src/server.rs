use iron::{Iron, Listening};

use db::{DB, DBMiddleware};
use config::{Config, ConfigMiddleware};
use error::AppResult;
use routes::create_router;

pub fn start(config: Config) -> AppResult<Listening> {
    let db = DBMiddleware::new(DB::new(&config.database_url)?);
    let config = ConfigMiddleware::new(config);

    let mut router = create_router();
    router.link_before(db);
    router.link_before(config);

    Iron::new(router).http("0.0.0.0:3000").map_err(Into::into)
}
