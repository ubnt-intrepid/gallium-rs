use iron::Chain;
use mount::Mount;
use app::{App, AppMiddleware};
use api;
use git;

pub fn create_handler(app: App) -> Chain {
    let mut mount = Mount::new();
    mount.mount("/", git::create_git_handler());
    mount.mount("/api/v1", api::v1::create_api_handler());

    let mut chain = Chain::new(mount);
    chain.link_before(AppMiddleware::new(app));

    chain
}
