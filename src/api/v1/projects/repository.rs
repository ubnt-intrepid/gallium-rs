use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use app::App;
use super::GetProjectInfoFromId;

pub fn get_file_list(req: &mut Request) -> IronResult<Response> {
    let (user, project) = req.get_project_from_id()?;

    let app = req.extensions.get::<App>().unwrap();
    let repo = app.get_repository(&user.username, &project.name).map_err(
        |err| {
            IronError::new(err, status::InternalServerError)
        },
    )?;
    let tree = repo.get_head_tree_objects().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    Ok(Response::with((status::Ok, JsonResponse::json(tree))))
}
