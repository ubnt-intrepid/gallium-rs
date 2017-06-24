use iron::prelude::*;
use iron::status;
use iron_json_response::JsonResponse;
use router::Router;
use git2::{self, Repository, ObjectType, Tree};
use serde_json::Value;
use diesel::prelude::*;
use app::App;
use models::{User, Project};
use schema::{users, projects};

pub fn get_file_list(req: &mut Request) -> IronResult<Response> {
    let app = req.extensions.get::<App>().unwrap();

    let id = req.get_id();
    let (user, project) = app.get_project_from_id(id)?;

    let repo = app.get_repository((user, project))?;
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


trait GetProjectFromId {
    fn get_project_from_id(&self, id: i32) -> IronResult<(User, Project)>;
}

impl GetProjectFromId for App {
    fn get_project_from_id(&self, id: i32) -> IronResult<(User, Project)> {
        let conn = self.get_db_conn().map_err(|err| {
            IronError::new(err, status::InternalServerError)
        })?;

        users::table
            .inner_join(projects::table)
            .filter(projects::dsl::id.eq(id))
            .get_result::<(User, Project)>(&*conn)
            .map_err(|err| IronError::new(err, status::InternalServerError))
    }
}


trait GetRepository {
    fn get_repository(&self, project_pair: (User, Project)) -> IronResult<Repository>;
}

impl GetRepository for App {
    fn get_repository(&self, project_pair: (User, Project)) -> IronResult<Repository> {
        let (user, project) = project_pair;
        let username = user.username;
        let proj_name = format!("{}.git", project.name);

        let repo_path = self.resolve_repository_path(&username, &proj_name)
            .map_err(|err| IronError::new(err, status::InternalServerError))?;
        Repository::open(repo_path).map_err(|err| IronError::new(err, status::InternalServerError))
    }
}


trait HeadTreeObjects {
    fn get_head_tree_objects(&self) -> Result<Vec<Value>, git2::Error>;
    fn collect_tree_object(&self, tree: &Tree) -> Vec<Value>;
}

impl HeadTreeObjects for Repository {
    fn get_head_tree_objects(&self) -> Result<Vec<Value>, git2::Error> {
        let head = self.head()?;
        let target = head.target().ok_or_else(|| git2::Error::from_str(""))?;
        let commit = self.find_commit(target)?;
        let tree = commit.tree()?;
        let objects = self.collect_tree_object(&tree);
        Ok(objects)
    }

    fn collect_tree_object(&self, tree: &Tree) -> Vec<Value> {
        tree.into_iter()
            .filter_map(|entry| {
                let kind = entry.kind().unwrap();
                match kind {
                    ObjectType::Blob => {
                        Some(json!({
                        "name": entry.name().unwrap(),
                        "filemode": format!("{:06o}", entry.filemode()),
                    }))
                    }
                    ObjectType::Tree => {
                        let child = self.collect_tree_object(&entry
                            .to_object(self)
                            .map(|o| o.into_tree().ok().unwrap())
                            .unwrap());
                        Some(json!({
                        "name": entry.name().unwrap(),
                        "filemode": format!("{:06o}", entry.filemode()),
                        "child": child,
                    }))
                    }
                    _ => None,
                }
            })
            .collect()
    }
}
