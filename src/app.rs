use iron::{Request, IronResult, BeforeMiddleware};

pub struct App {}

pub struct AppMiddleware {}

impl BeforeMiddleware for AppMiddleware {
    fn before(&self, _req: &mut Request) -> IronResult<()> {
        Ok(())
    }
}
