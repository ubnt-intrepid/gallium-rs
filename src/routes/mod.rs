mod api;
mod git;

use mount::Mount;
use config::Config;

pub fn create_handler(_config: Config) -> Mount {
    let git_router = git::create_git_handler();
    let api_router = api::create_api_handler();

    let mut mount = Mount::new();
    mount.mount("/", git_router);
    mount.mount("/api/v1", api_router);

    mount
}
