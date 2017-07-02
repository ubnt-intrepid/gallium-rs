mod git;
mod api {
    #[path = "v1/mod.rs"]
    pub mod v1;
}

use iron::Chain;
use mount::Mount;


header! {
    (WWWAuthenticate, "WWW-Authenticate") => [String]
}

pub fn create_router() -> Chain {
    let mut mount = Mount::new();
    mount.mount("/", git::create_git_router());
    mount.mount("/api/v1", api::v1::create_api_router());
    Chain::new(mount)
}
