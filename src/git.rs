use std::path::{Path, PathBuf};

use clap::ArgEnum;
use git2::{IndexAddOption, PushOptions};

type Result<T> = std::result::Result<T, git2::Error>;

const HEAD: &str = "HEAD";

/// The authentication method being used.
#[derive(ArgEnum, Clone)]
pub enum AuthenticationMethod {
    /// `ssh-agent`.
    SshAgent,
    /// SSH key containing the path and passphrase.
    SshKey { path: PathBuf, passphrase: String },
}

/// Trait abstracting over a repository backend.
pub trait Repository: Send {
    /// Stage a file path.
    fn stage<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>;

    /// Stage all files.
    fn stage_all(&self) -> Result<()>;

    /// Commit staged files with a message.
    fn commit(&self, message: &str) -> Result<()>;

    /// Push commits to the remote.
    fn push(&self, authentication_method: AuthenticationMethod) -> Result<()>;
}

/// Wrapper over `git2::Repository`.
pub struct WatchedRepository(git2::Repository);

impl WatchedRepository {
    /// Create a `WatchedRepository` from a given path.
    pub fn new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self(git2::Repository::open(path)?))
    }
}

impl Repository for WatchedRepository {
    /// Stage a single path.
    fn stage<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        // TODO: find a way to handle the unwraps cleanly
        let mut index = self.0.index()?;
        index.add_path(
            path.as_ref()
                .strip_prefix(self.0.path().parent().unwrap())
                .unwrap(),
        )?;
        index.write()?;
        Ok(())
    }

    /// Stage all paths.
    fn stage_all(&self) -> Result<()> {
        let mut index = self.0.index()?;
        index.add_all(["*"].iter(), IndexAddOption::CHECK_PATHSPEC, None)?;
        index.write()?;
        Ok(())
    }

    /// Commit the staged paths with the provided message.
    fn commit(&self, message: &str) -> Result<()> {
        let repo = &self.0;
        // Find the current tree
        let tree_oid = repo.index()?.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;
        // Find the commit "metadata" (i.e. author, etc)
        let config = repo.config()?;
        let name = config.get_string("user.name")?;
        let email = config.get_string("user.email")?;
        let signature = git2::Signature::now(&name, &email)?;
        // Get the parent commit
        let parent_commit = repo.head()?.resolve()?.peel_to_commit()?;
        // Perform the actual commit
        repo.commit(
            Some(HEAD),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;
        Ok(())
    }

    /// Pushes the current branch into "origin".
    /// The function relies on `ssh-agent` for git authentication.
    fn push(&self, authentication_method: AuthenticationMethod) -> Result<()> {
        let repo = &self.0;

        // TODO: allow remote to be configurable
        let mut remote = repo.find_remote("origin")?;

        let head = repo.head()?;
        let refspecs: &[&str] = &[head.name().unwrap()];

        let mut remote_callbacks = git2::RemoteCallbacks::new();

        match authentication_method {
            AuthenticationMethod::SshAgent => {
                remote_callbacks.credentials(|_url, username_from_url, _allowed_types| {
                    git2::Cred::ssh_key_from_agent(username_from_url.unwrap())
                });
            }
            AuthenticationMethod::SshKey {
                path: private_key_path,
                passphrase: key_passphrase,
            } => {
                remote_callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                    git2::Cred::ssh_key(
                        username_from_url.unwrap(),
                        Some(&private_key_path.clone().with_extension("pub")),
                        &private_key_path,
                        Some(&key_passphrase.clone()),
                    )
                });
            }
        };

        remote_callbacks.push_update_reference(|refname, status| {
            if let Some(status_message) = status {
                log::error!("error pushing reference {}", refname);
                Err(git2::Error::from_str(status_message))
            } else {
                Ok(())
            }
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(remote_callbacks);

        remote.push(refspecs, Some(&mut push_options))?;
        Ok(())
    }
}

/// Dummy repository, mainly useful for testing.
pub struct DummyRepository;

impl Repository for DummyRepository {
    /// Stage a single path.
    fn stage<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        log::info!("staged file {}", path.as_ref().display());
        Ok(())
    }

    /// Stage all paths.
    fn stage_all(&self) -> Result<()> {
        log::info!("staged all files");
        Ok(())
    }

    /// Commit the staged paths with the provided message.
    fn commit(&self, message: &str) -> Result<()> {
        log::info!("commited staged files with message: {}", message);
        Ok(())
    }

    /// Push the commits to the remote.
    fn push(&self, _authentication_method: AuthenticationMethod) -> Result<()> {
        log::info!("pushed files to remote");
        Ok(())
    }
}
