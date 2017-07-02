use std::fs;
use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use git2;
use serde_json::Value as JsonValue;
use users::get_user_by_name;
use error::AppResult;


pub struct Repository {
    inner: git2::Repository,
}

impl Repository {
    pub fn create<P: AsRef<Path>>(repo_path: P) -> AppResult<Self> {
        let repo_path_str = repo_path.as_ref().to_str().unwrap();

        // Get uid/gid
        let user = get_user_by_name("git").unwrap();
        let uid = user.uid();
        let gid = user.primary_group_id();

        // Create destination directory of repository.
        Command::new("/bin/mkdir")
            .args(&["-p", repo_path_str])
            .uid(uid)
            .gid(gid)
            .spawn()
            .and_then(|mut ch| ch.wait())
            .and_then(|st| if st.success() {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "cannot change owner of repository",
                ))
            })?;

        // Initialize git repository
        let status = Command::new("/usr/bin/git")
            .args(&["init", "--bare", repo_path_str])
            .current_dir(&repo_path)
            .uid(uid)
            .gid(gid)
            .spawn()
            .and_then(|mut ch| ch.wait())?;
        if !status.success() {
            return Err(
                io::Error::new(
                    io::ErrorKind::Other,
                    "`git init` exited with non-zero status",
                ).into(),
            );
        }

        Self::open(&repo_path).map_err(Into::into)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        let inner = git2::Repository::open(path)?;
        Ok(Repository { inner })
    }

    pub fn path(&self) -> &Path {
        self.inner.path()
    }

    pub fn get_head_tree_objects(&self) -> AppResult<Vec<JsonValue>> {
        let head = self.inner.head()?;
        let target = head.target().ok_or_else(|| git2::Error::from_str(""))?;
        let commit = self.inner.find_commit(target)?;
        let tree = commit.tree()?;
        let objects = self.collect_tree_object(&tree);
        Ok(objects)
    }

    fn collect_tree_object(&self, tree: &git2::Tree) -> Vec<JsonValue> {
        tree.into_iter()
            .filter_map(|entry| {
                let type_ = match entry.kind().unwrap() {
                    git2::ObjectType::Blob => "blob",
                    git2::ObjectType::Tree => "tree",
                    _ => return None,
                };
                Some(json!({
                    "id": entry.id().to_string(),
                    "name": entry.name().unwrap(),
                    "type": type_,
                    "filemode": format!("{:06o}", entry.filemode()),
                }))
            })
            .collect()
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
            let message = format!("`git {}` was exited with non-zero status: {}", service, String::from_utf8_lossy(&output.stderr));
            Err(io::Error::new(io::ErrorKind::Other, message).into())
        }
    }

    pub fn remove(self) -> Result<(), (Self, io::Error)> {
        fs::remove_dir_all(self.inner.path()).map_err(|err| (self, err))
    }
}
