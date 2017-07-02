use std::fs;
use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use git2;
use serde_json::Value as JsonValue;
use users::get_user_by_name;
use error::{AppResult, AppError};


pub struct Repository {
    inner: git2::Repository,
}

impl Repository {
    pub(super) fn init<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        // Get uid/gid
        let user = get_user_by_name("git").unwrap();
        let uid = user.uid();
        let gid = user.primary_group_id();

        // Create destination directory of repository.
        fs::create_dir_all(path.as_ref())?;
        let status = Command::new("/bin/chown")
            .args(&["-R", "git:git"])
            .arg(path.as_ref())
            .spawn()
            .and_then(|mut ch| ch.wait())?;
        if !status.success() {
            return Err(AppError::from(
                "failed to change permission of registory directory",
            ));
        }

        // Initialize git repository
        let status = Command::new("/usr/bin/git")
            .args(&["init", "--bare"])
            .current_dir(&path)
            .uid(uid)
            .gid(gid)
            .spawn()
            .and_then(|mut ch| ch.wait())?;
        if !status.success() {
            return Err(AppError::from("`git init` exited with non-zero status"));
        }

        let inner = git2::Repository::open(path)?;
        Ok(Repository { inner })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        let inner = git2::Repository::open(path)?;
        Ok(Repository { inner })
    }

    pub fn remove(self) -> Result<(), (Self, io::Error)> {
        fs::remove_dir_all(self.inner.path()).map_err(|err| (self, err))
    }

    pub fn path(&self) -> &Path {
        self.inner.path()
    }

    pub fn list_tree(&self, refname: &str, is_recursive: bool) -> AppResult<Vec<JsonValue>> {
        let reference = self.inner.find_reference(refname)?.resolve()?;
        let target = reference.target().ok_or_else(|| {
            AppError::from("failed to get target object")
        })?;
        let commit = self.inner.find_commit(target)?;
        let tree = commit.tree()?;

        let mut objects = Vec::new();
        walk_tree(
            &self.inner,
            &tree,
            &mut objects,
            PathBuf::default(),
            is_recursive,
        )?;

        Ok(objects)
    }

    pub fn run_rpc_command<'a>(&self, service: &str, stdin: Option<&mut Box<Read + 'a>>) -> AppResult<Vec<u8>> {
        let args: Vec<&str> = if stdin.is_some() {
            vec![service, "--stateless-rpc", "."]
        } else {
            vec![service, "--stateless-rpc", "--advertise-refs", "."]
        };

        let mut child = Command::new("/usr/bin/git")
            .args(args)
            .current_dir(self.inner.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        if let Some(stdin) = stdin {
            io::copy(stdin, child.stdin.as_mut().unwrap())?;
        }

        let output = child.wait_with_output()?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            let message = format!(
                "`git {}` was exited with non-zero status: {}",
                service,
                String::from_utf8_lossy(&output.stderr)
            );
            Err(io::Error::new(io::ErrorKind::Other, message).into())
        }
    }
}


fn walk_tree(
    repo: &git2::Repository,
    tree: &git2::Tree,
    dest: &mut Vec<JsonValue>,
    path: PathBuf,
    is_recursive: bool,
) -> AppResult<()> {
    for entry in tree {
        let id = entry.id().to_string();
        let name = entry.name().ok_or_else(
            || AppError::from("Failed to get entry name"),
        )?;
        let filemode = entry.filemode();
        let path = path.join(&name);

        let type_ = match entry.kind() {
            Some(git2::ObjectType::Blob) => "blob",
            Some(git2::ObjectType::Tree) => "tree",
            _ => return Err(AppError::from("Invalid kind")),
        };
        if is_recursive && type_ == "tree" {
            let tree = entry
                .to_object(repo)
                .ok()
                .and_then(|o| o.into_tree().ok())
                .unwrap();
            walk_tree(repo, &tree, dest, path, is_recursive)?;
        } else {
            let item = json!({
                "id": id,
                "type": type_,
                "name": &name,
                "path": path,
                "filemode": format!("{:06o}", filemode),
            });
            dest.push(item);
        }
    }
    Ok(())
}
