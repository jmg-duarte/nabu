use std::path::Path;

use git2::PushOptions;

type Result<T> = std::result::Result<T, git2::Error>;

const HEAD: &str = "HEAD";

pub struct WatchedRepository(git2::Repository);

impl WatchedRepository {
    /// Create a `WatchedRepository` from a given path.
    pub fn new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self(git2::Repository::open(path)?))
    }

    pub fn stage<P>(&self, path: P) -> Result<()>
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
    }

    pub fn commit(&self, message: &str) -> Result<()> {
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

    #[allow(unused)] // TODO: use this function when closing the program
    pub fn push(&self) -> Result<()> {
        let repo = &self.0;
        let head = repo.head()?;
        let current_branch_name = head.name().unwrap();
        let mut remote = repo.find_remote(&current_branch_name)?;

        let mut remote_callbacks = git2::RemoteCallbacks::new();
        remote_callbacks.push_update_reference(|refname, status| {
            println!("{:#?} {:#?}", refname, status);
            Ok(())
        });

        let refspecs: &[&str] = &[];
        let mut push_options = PushOptions::new();
        remote.push(refspecs, Some(&mut push_options))?;
        Ok(())
    }
}
