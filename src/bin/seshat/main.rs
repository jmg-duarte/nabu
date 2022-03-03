use seshat::{fs::list_subdirs, git::WatchedRepository};

use std::{
    collections::HashSet,
    env::current_dir,
    ffi::OsStr,
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

use clap::{Args, Parser, Subcommand};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};

const DEFAULT_DELAY: u64 = 5;

fn default_config_path() -> String {
    std::env::var("HOME").unwrap() + "/.config/seshat.toml"
}

fn current_dir_string() -> String {
    String::from(current_dir().unwrap().to_str().unwrap())
}

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Watch(WatchArgs),
}

#[derive(Args)]
struct WatchArgs {
    /// The directory to watch over.
    #[clap(default_value_t=current_dir_string())]
    directory: String,
    /// Whether to watch sub-directories.
    #[clap(short, long)]
    recursive: bool,
    /// Path to the configuration file.
    #[clap(short, long, default_value_t=default_config_path())]
    config: String,
    /// Watch over directory and print commands not performing them.
    #[clap(long)]
    dry_run: bool,
    #[clap(long, default_value_t=DEFAULT_DELAY)]
    delay: u64,
}

macro_rules! handle_event {
    ($path:ident, $message:literal) => {
        (
            $path,
            format!($message, $path.to_str().unwrap(), chrono::Utc::now()),
        )
    };
}

struct Watch {
    args: WatchArgs,
    repo: WatchedRepository,
}

impl From<WatchArgs> for Watch {
    fn from(args: WatchArgs) -> Self {
        let directory = args.directory.clone();
        Self {
            args,
            repo: WatchedRepository::new(directory).unwrap(),
        }
    }
}

impl Watch {
    fn run(self) {
        let (tx, rx): (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(self.args.delay)).unwrap();

        let directories = list_subdirs(
            &self.args.directory,
            HashSet::from_iter(vec![".obsidian", ".git"].into_iter().map(|s| OsStr::new(s))),
        );

        for dir in &directories {
            watcher.watch(dir, RecursiveMode::NonRecursive).unwrap();
        }

        println!("Watching over {:#?}", directories);

        loop {
            match rx.recv() {
                Ok(event) => self.handle_event(&event, &self.repo),
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }

    fn handle_event(&self, event: &DebouncedEvent, repo: &WatchedRepository) {
        println!("{:?}", event);
        // TODO: better commit messages (e.g. short title, descriptive body)
        // TODO: configurable commit messages
        let (path, message) = match event {
            DebouncedEvent::Create(path) => handle_event!(path, "created file {} @ {}"),
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

fn main() {
    let cli = Cli::parse();
    match cli.commands {
        Commands::Watch(args) => Watch::from(args).run(),
    }
}
