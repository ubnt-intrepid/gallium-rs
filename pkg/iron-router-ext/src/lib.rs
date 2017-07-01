extern crate iron;
extern crate router;

use iron::Chain;
use iron::method::Method;
use router::Router;


pub trait Route {
    fn route_path() -> &'static str;
    fn route_method() -> Method;
    fn route_id() -> &'static str;
}

pub trait RegisterRoute {
    fn register<R: Route + Into<Chain>>(&mut self, route: R) -> &mut Self;
}

impl RegisterRoute for Router {
    fn register<R: Route + Into<Chain>>(&mut self, route: R) -> &mut Self {
        self.route(
            R::route_method(),
            R::route_path(),
            route.into(),
            R::route_id(),
        )
    }
}
