mod oauth;
mod git;
mod api {
    #[path = "v1/mod.rs"]
    pub mod v1;
}

use iron::Chain;
use mount::Mount;
use error::AppResult;
use middlewares::app::AppMiddleware;
use config::Config;

header! {
    (WWWAuthenticate, "WWW-Authenticate") => [String]
}

pub fn create_handler() -> AppResult<Chain> {
    let config = Config::load()?;
    let app = AppMiddleware::new(config)?;

    let mut mount = Mount::new();
    mount.mount("/", git::create_git_handler());
    mount.mount("/api/v1", api::v1::create_api_handler());
    mount.mount("/oauth", oauth::create_oauth_handler());

    let mut chain = Chain::new(mount);
    chain.link_before(app);
    Ok(chain)
}
