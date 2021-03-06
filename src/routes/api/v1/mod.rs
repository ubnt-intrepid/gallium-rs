mod error;
mod response;

mod ssh_keys;
mod projects;
mod repository;
mod users;

use router::Router;
use iron_router_ext::RegisterRoute;

pub fn create_api_router() -> Router {
    let mut router = Router::new();
    router.register(projects::GetProjects);
    router.register(projects::GetProject);
    router.register(projects::CreateProject);
    router.register(projects::DeleteProject);
    router.register(repository::ShowTree);
    router.register(repository::GetBlob);
    router.register(repository::GetRawBlob);
    router.register(ssh_keys::GetKeys);
    router.register(ssh_keys::GetKey);
    router.register(ssh_keys::AddKey);
    router.register(ssh_keys::DeleteKey);
    router.register(users::GetUsers);
    router.register(users::GetUser);
    router.register(users::CreateUser);
    router
}
