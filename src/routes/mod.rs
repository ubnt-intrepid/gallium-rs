mod oauth;
mod git;

use iron::Chain;
use mount::Mount;
use api;
use app::AppMiddleware;

header! {
    (WWWAuthenticate, "WWW-Authenticate") => [String]
}

pub fn create_handler(app: AppMiddleware) -> Chain {
    let mut mount = Mount::new();
    mount.mount("/", git::create_git_handler());
    mount.mount("/api/v1", api::v1::create_api_handler());
    mount.mount("/oauth", oauth::create_oauth_handler());

    let mut chain = Chain::new(mount);
    chain.link_before(app);
    chain
}
