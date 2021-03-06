use r2d2::{Pool, PooledConnection, GetTimeout};
use r2d2_diesel::ConnectionManager;
use diesel::pg::PgConnection;
use error::AppResult;
use iron::{Request, IronResult, BeforeMiddleware};
use iron::typemap::Key;


pub type PooledDbConnection = PooledConnection<ConnectionManager<PgConnection>>;


#[derive(Clone)]
pub struct DB(Pool<ConnectionManager<PgConnection>>);

impl DB {
    pub fn new(database_url: &str) -> AppResult<Self> {
        let manager = ConnectionManager::new(database_url);
        let pool = Pool::new(Default::default(), manager)?;
        Ok(DB(pool))
    }

    pub fn from_req(req: &mut Request) -> Result<PooledDbConnection, GetTimeout> {
        let db = req.extensions.get::<Self>().unwrap();
        db.0.get()
    }

    pub fn get_db_conn(&self) -> Result<PooledDbConnection, GetTimeout> {
        self.0.get()
    }
}

impl Key for DB {
    type Value = DB;
}


pub struct DBMiddleware(DB);

impl DBMiddleware {
    pub fn new(db: DB) -> Self {
        DBMiddleware(db)
    }
}

impl BeforeMiddleware for DBMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<DB>(self.0.clone());
        Ok(())
    }
}
