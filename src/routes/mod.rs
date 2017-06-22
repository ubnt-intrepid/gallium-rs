mod api;
mod git;

use iron::Chain;
use mount::Mount;
use config::Config;
use app::{App, AppMiddleware};

pub fn create_handler(config: Config) -> Chain {
    let app = App::new(config).unwrap();

    let mut mount = Mount::new();
    mount.mount("/", git::create_git_handler());
    mount.mount("/api/v1", api::create_api_handler());

    let mut chain = Chain::new(mount);
    chain.link_before(AppMiddleware::new(app));

    chain
}
