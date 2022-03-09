use nabu::{
    config::{global_config_path, Config},
    fs::list_subdirs,
    git::WatchedRepository,
};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, RecvTimeoutError, Sender},
        Arc,
    },
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
    #[clap(long)]
    push_on_exit: bool,
}

impl WatchArgs {
    fn update_with_config(&mut self, config: &Config) {
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
}

pub(crate) struct Watch {
    args: WatchArgs,
    repo: WatchedRepository,
    running: Arc<AtomicBool>,
}

impl Watch {
    pub fn new(args: WatchArgs, running: Arc<AtomicBool>) -> Result<Self> {
        let directory = args.directory.clone().canonicalize()?;
        info!("{}", directory.display());
        Ok(Self {
            args,
            repo: WatchedRepository::new(directory)?,
            running,
        })
    }

    pub fn run(mut self) {
        let mut local_config_path = self.args.directory.clone();
        local_config_path.push("nabu.toml");

        let config = Config::from_path(&local_config_path)
            .or_else(|_| Config::from_path(global_config_path()))
            .unwrap_or(Config::default());

        self.args.update_with_config(&config);

        let (tx, rx): (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(self.args.delay.unwrap())).unwrap();

        let ignored_set = self
            .args
            .ignore
            .iter()
            .map(|s| OsStr::new(s))
            .collect::<HashSet<&OsStr>>();

        let directories = list_subdirs(&self.args.directory, ignored_set);

        for dir in &directories {
            watcher.watch(dir, RecursiveMode::NonRecursive).unwrap();
        }

        debug!("watching over {:?}", directories);

        while self.running.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(event) => {
                    debug!("event received: {:?}", &event);
                    self.handle_event(&event, &self.repo)
                }
                Err(RecvTimeoutError::Disconnected) => error!("sender disconnected"),
                _ => {}
            }
        }

        self.repo.stage_all().unwrap();
        self.repo
            .commit(&format!("nabu exited snapshot @ {}", chrono::Utc::now()))
            .unwrap();

        if self.args.push_on_exit {
            match self.repo.push() {
                Ok(()) => {
                    info!("successfully pushed");
                }
                Err(err) => {
                    warn!("{}", err.message());
                }
            }
        }
    }

    fn handle_event(&self, event: &DebouncedEvent, repo: &WatchedRepository) {
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
