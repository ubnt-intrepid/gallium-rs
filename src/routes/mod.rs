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
use iron_json_response::JsonResponseMiddleware;


header! {
    (WWWAuthenticate, "WWW-Authenticate") => [String]
}

pub fn create_handler(config: Config) -> AppResult<Chain> {
    let app = AppMiddleware::new(config)?;

    let mut mount = Mount::new();
    mount.mount("/", git::create_git_handler());
    mount.mount("/api/v1", api::v1::create_api_handler());

    let mut chain = Chain::new(mount);
    chain.link_before(app);
    chain.link_after(JsonResponseMiddleware::new());
    Ok(chain)
}
