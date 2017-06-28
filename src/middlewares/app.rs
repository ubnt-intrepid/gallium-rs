use std::sync::Arc;
use iron::{Request, IronResult, BeforeMiddleware};
use config::Config;
use error::AppResult;
use db::DB;


pub struct AppMiddleware {
    config: Arc<Config>,
    db: DB,
}

impl AppMiddleware {
    pub fn new(config: Config) -> AppResult<Self> {
        let db = DB::new(&config.database_url)?;
        Ok(AppMiddleware {
            config: Arc::new(config),
            db,
        })
    }
}

impl BeforeMiddleware for AppMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<DB>(self.db.clone());
        req.extensions.insert::<Config>(self.config.clone());
        Ok(())
    }
}
