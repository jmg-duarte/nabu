use nabu::{
    config::{global_config_path, Config, DEFAULT_DELAY},
    fs::list_subdirs,
    git::{AuthenticationMethod, DummyRepository, Repository, WatchedRepository},
};

use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use clap::Args;
use color_eyre::Result;
use log::{debug, error, info, warn};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};

macro_rules! handle_event {
    ($path:ident, $message:literal) => {{
        let commit_message = format!($message, $path.to_str().unwrap(), chrono::Utc::now());
        ::log::info!("commit with message: {}", commit_message);
        ($path, commit_message)
    }};
}

const AUTHENTICATION_METHOD_GROUP_NAME: &str = "authentication_method_group";
const HTTPS_GROUP_NAME: &str = "https_group";
const SSH_KEY_GROUP_NAME: &str = "ssh_key_group";
const PUSH_GROUP_NAME: &str = "push_group";

#[derive(Args)]
pub(crate) struct WatchArgs {
    /// The directory to watch over.
    #[clap(parse(from_os_str))]
    directory: PathBuf,

    /// Whether to watch sub-directories.
    #[clap(short, long)]
    recursive: bool,

    /// Watch over directory and print commands not performing them.
    #[clap(long)]
    dry_run: bool,

    /// Watcher event delay.
    #[clap(long)]
    delay: Option<u64>,

    /// List of directories to ignore.
    #[clap(long)]
    ignore: Vec<String>,

    /// Path to the configuration file.
    #[clap(short, long, parse(from_os_str))]
    config: Option<PathBuf>,

    /// Whether to push on exit.
    /// If not set, the value will be read from the config.
    #[clap(long, group(PUSH_GROUP_NAME))]
    push_on_exit: bool,

    /// Use the ssh-agent as authenticaton method.
    #[clap(
        long,
        group(AUTHENTICATION_METHOD_GROUP_NAME),
        requires(PUSH_GROUP_NAME)
    )]
    ssh_agent: bool,

    /// Use the ssh-key as authentication method.
    #[clap(
        long,
        parse(from_os_str),
        requires(PUSH_GROUP_NAME),
        groups(&[AUTHENTICATION_METHOD_GROUP_NAME, SSH_KEY_GROUP_NAME]),
    )]
    ssh_key: Option<PathBuf>,

    /// Provide a passphrase for the ssh-key.
    #[clap(long, requires(SSH_KEY_GROUP_NAME), default_value_t)]
    ssh_passphrase: String,

    /// Use https as the authentication method.
    /// Requires a username and password.
    #[clap(
        long,
        requires(PUSH_GROUP_NAME),
        groups(&[AUTHENTICATION_METHOD_GROUP_NAME, HTTPS_GROUP_NAME]),
    )]
    https: bool,

    /// Https username.
    #[clap(short = 'u', long = "username", requires(HTTPS_GROUP_NAME))]
    https_username: Option<String>,

    /// Https password.
    #[clap(short = 'p', long = "password", requires(HTTPS_GROUP_NAME))]
    https_password: Option<String>,
}

impl WatchArgs {
    pub fn run(mut self, watching: Arc<AtomicBool>) -> Result<()> {
        self.update_from_config();
        let watched_directories = self.list_watched_directories();
        let delay = self.delay.unwrap_or(DEFAULT_DELAY);
        if self.dry_run {
            Watch::new(
                DummyRepository,
                watching,
                watched_directories,
                delay,
                self.push_on_exit,
                self.get_authentication_method().unwrap(),
            )
            .run();
        } else {
            let directory = self.directory.clone().canonicalize()?;
            info!("{}", directory.display());
            let repo = WatchedRepository::new(directory)?;
            Watch::new(
                repo,
                watching,
                watched_directories,
                delay,
                self.push_on_exit,
                self.get_authentication_method().unwrap(),
            )
            .run();
        }
        Ok(())
    }

    pub fn update_from_config(&mut self) {
        let config = if let Some(path) = &self.config {
            Config::from_path(path).unwrap()
        } else {
            let mut local_config_path = self.directory.clone();
            local_config_path.push("nabu.toml");

            Config::from_path(&local_config_path)
                .or_else(|_| Config::from_path(global_config_path()))
                .unwrap_or_default()
        };

        if self.delay.is_none() {
            self.delay = Some(config.delay);
        }

        if self.ignore.is_empty() {
            self.ignore = config.ignore.clone();
        }

        if !self.push_on_exit {
            self.push_on_exit |= config.push_on_exit;
        }
    }

    pub fn list_watched_directories(&self) -> Vec<PathBuf> {
        let ignored_set = self
            .ignore
            .iter()
            .map(OsStr::new)
            .collect::<HashSet<&OsStr>>();

        list_subdirs(&self.directory, ignored_set)
    }

    pub fn get_authentication_method(&self) -> Result<AuthenticationMethod> {
        if self.ssh_agent {
            if env::var("SSH_AGENT_PID").is_err() && env::var("SSH_AUTH_SOCK").is_err() {
                warn!("ssh-agent is not running.");
            }
            return Ok(AuthenticationMethod::SshAgent);
        }

        if let Some(path) = self.ssh_key.clone() {
            return if path.exists() {
                Ok(AuthenticationMethod::SshKey {
                    path,
                    passphrase: self.ssh_passphrase.clone(),
                })
            } else {
                // TODO create a proper error
                panic!("provided key does not exist")
            };
        }

        return Ok(AuthenticationMethod::Https {
            username: self.https_username.clone().unwrap(),
            password: self.https_password.clone().unwrap(),
        });
    }
}

pub(crate) struct Watch<R>
where
    R: Repository,
{
    repo: R,
    running: Arc<AtomicBool>,
    watchlist: Vec<PathBuf>,
    delay: u64,
    push_on_exit: bool,
    authentication_method: AuthenticationMethod,
}

impl<R> Watch<R>
where
    R: Repository + 'static,
{
    pub fn new(
        repo: R,
        running: Arc<AtomicBool>,
        watchlist: Vec<PathBuf>,
        delay: u64,
        push_on_exit: bool,
        authentication_method: AuthenticationMethod,
    ) -> Self {
        Self {
            repo,
            running,
            watchlist,
            delay,
            push_on_exit,
            authentication_method,
        }
    }

    pub fn run(self) {
        let (event_snd, event_rcv): (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) = channel();
        let mut watcher = watcher(event_snd, Duration::from_secs(self.delay)).unwrap();

        for dir in &self.watchlist {
            info!("adding {} to watcher", dir.display());
            watcher.watch(dir, RecursiveMode::NonRecursive).unwrap();
        }

        debug!("watching over {:?}", &self.watchlist);

        while self.running.load(Ordering::SeqCst) {
            match event_rcv.recv_timeout(Duration::from_millis(500)) {
                Ok(event) => {
                    debug!("event received: {:?}", &event);
                    self.handle_event(&event, &self.repo)
                }
                Err(RecvTimeoutError::Disconnected) => error!("sender disconnected"),
                _ => {}
            }
        }

        info!("Termination signal received, attempting to save changes.");

        self.repo.stage_all().unwrap();
        info!("Staged changes.");
        self.repo
            .commit(&format!("nabu exited snapshot @ {}", chrono::Utc::now()))
            .unwrap();

        info!("Commited changes.");

        if self.push_on_exit {
            let (sig_snd, sig_rcv) = channel();
            let repo = Arc::new(Mutex::new(self.repo));
            thread::spawn(move || {
                let r = repo.try_lock().unwrap();
                match r.push(self.authentication_method) {
                    Ok(()) => {
                        info!("Successfully pushed to remote.");
                    }
                    Err(err) => {
                        warn!("{}", err.message());
                    }
                }
                sig_snd.send(()).unwrap();
            });
            if let Err(_) = sig_rcv.recv_timeout(Duration::from_secs(5)) {
                warn!("Timeout while pushing, cleaning up now.");
            }
        }
    }

    fn handle_event(&self, event: &DebouncedEvent, repo: &R)
    where
        R: Repository,
    {
        debug!("received event: {:?}", event);
        // TODO: better commit messages (e.g. short title, descriptive body)
        // TODO: configurable commit messages
        let (path, message) = match event {
            DebouncedEvent::Create(path) => {
                if path.is_dir() {
                    return;
                }
                handle_event!(path, "created file {} @ {}")
            }
            DebouncedEvent::Write(path) => handle_event!(path, "written file {} @ {}"),
            DebouncedEvent::Chmod(path) => handle_event!(path, "chmod file {} @ {}"),
            DebouncedEvent::Remove(path) => handle_event!(path, "deleted file {} @ {}"),
            DebouncedEvent::Rename(old, new) => (
                new,
                format!(
                    "renamed file {} to {} @ {}",
                    old.to_str().unwrap(),
                    new.to_str().unwrap(),
                    chrono::Utc::now()
                ),
            ),
            // TODO: handle these two later
            DebouncedEvent::Rescan => todo!(),
            DebouncedEvent::Error(_, _) => todo!(),
            DebouncedEvent::NoticeRemove(_) | DebouncedEvent::NoticeWrite(_) => {
                return;
            }
        };

        repo.stage(path).unwrap();
        repo.commit(&message).unwrap();
    }
}
