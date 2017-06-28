use r2d2::{Pool, PooledConnection, GetTimeout};
use r2d2_diesel::ConnectionManager;
use diesel::pg::PgConnection;
use error::AppResult;
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

    pub fn get_db_conn(&self) -> Result<PooledDbConnection, GetTimeout> {
        self.0.get()
    }
}

impl Key for DB {
    type Value = DB;
}
