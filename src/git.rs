use git2::{
    Commit, Cred, Direction, ErrorClass, ErrorCode, ObjectType, Oid, PushOptions, RemoteCallbacks,
    Repository,
};

use std::env;
use std::path::Path;

pub trait GitManagement {
    fn init(&mut self, repo_path: &str) -> Result<(), git2::Error>;
    fn checkout_branch(&self, branch_name: &str) -> Result<(), git2::Error>;
    fn add(&self) -> Result<(), git2::Error>;
    fn commit(&self, subject: &str) -> Result<Oid, git2::Error>;
    fn push(&self, branch_name: &str) -> Result<(), git2::Error>;
}

pub struct Git {
    repo: Option<Repository>,
}

impl Default for Git {
    fn default() -> Self {
        Self { repo: None }
    }
}

impl GitManagement for Git {
    fn init(&mut self, repo_path: &str) -> Result<(), git2::Error> {
        Repository::open(&Path::new(&repo_path)).map(|repo| self.repo = Some(repo))
    }

    fn checkout_branch(&self, branch_name: &str) -> Result<(), git2::Error> {
        let repo = self.repo.as_ref().unwrap();

        let commit = repo
            .head()
            .map(|head| head.target())
            .and_then(|oid| repo.find_commit(oid.unwrap()))?;

        // Create new branch if it doesn't exist
        match repo.branch(branch_name, &commit, false) {
            // This command can fail due to an existing reference. This error should be ignored.
            Err(err)
                if !(err.class() == ErrorClass::Reference && err.code() == ErrorCode::Exists) =>
            {
                return Err(err);
            }
            _ => {}
        }

        let refname = format!("refs/heads/{}", branch_name);
        let obj = repo.revparse_single(&*refname)?;

        repo.checkout_tree(&obj, None)?;

        repo.set_head(&*refname)
    }

    fn add(&self) -> Result<(), git2::Error> {
        let mut index = self.repo.as_ref().unwrap().index()?;

        index.add_path(Path::new("README.md"))?;
        index.write()
    }

    fn commit(&self, subject: &str) -> Result<Oid, git2::Error> {
        let repo = self.repo.as_ref().unwrap();
        let mut index = repo.index()?;

        let signature = repo.signature()?; // Use default user.name and user.email

        let oid = index.write_tree()?;
        let parent_commit = self.find_last_commit()?;
        let tree = repo.find_tree(oid)?;

        repo.commit(
            Some("HEAD"),      // point HEAD to our new commit
            &signature,        // author
            &signature,        // committer
            &subject,          // commit message
            &tree,             // tree
            &[&parent_commit], // parent commit
        )
    }

    fn push(&self, branch_name: &str) -> Result<(), git2::Error> {
        let mut remote = self.repo.as_ref().unwrap().find_remote("origin")?;

        remote.connect_auth(Direction::Push, Some(self.get_callbacks()), None)?;

        let mut options = PushOptions::default();
        options.remote_callbacks(self.get_callbacks());
        remote.push(
            &[format!(
                "refs/heads/{}:refs/heads/{}",
                branch_name, branch_name
            )],
            Some(&mut options),
        )
    }
}

impl Git {
    fn find_last_commit(&self) -> Result<Commit, git2::Error> {
        let obj = self
            .repo
            .as_ref()
            .unwrap()
            .head()?
            .resolve()?
            .peel(ObjectType::Commit)?;
        obj.into_commit()
            .map_err(|_| git2::Error::from_str("Couldn't find commit"))
    }

    fn get_callbacks<'a>(&self) -> RemoteCallbacks<'a> {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username, _allowed_types| {
            Cred::ssh_key(
                username.unwrap(),
                None,
                std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                None,
            )
        });
        callbacks
    }
}
