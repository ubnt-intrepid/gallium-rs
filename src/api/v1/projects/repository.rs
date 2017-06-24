use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;
use app::App;

pub fn get_file_list(req: &mut Request) -> IronResult<Response> {
    let app = req.extensions.get::<App>().unwrap();

    let id = req.get_id();
    let (user, project) = app.get_project_from_id(id).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    let repo = app.get_repository((user, project)).map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;
    let tree = repo.get_head_tree_objects().map_err(|err| {
        IronError::new(err, status::InternalServerError)
    })?;

    Ok(Response::with((status::Ok, JsonResponse::json(tree))))
}


trait GetIdentifier {
    fn get_id(&self) -> i32;
}

impl<'a, 'b: 'a> GetIdentifier for Request<'a, 'b> {
    fn get_id(&self) -> i32 {
        let router = self.extensions.get::<Router>().unwrap();
        router.find("id").and_then(|s| s.parse().ok()).unwrap()
    }
}
