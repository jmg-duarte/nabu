mod fs;
mod git;

use std::{
    collections::HashSet,
    env::current_dir,
    ffi::OsStr,
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

use clap::{Args, Parser, Subcommand};
use fs::list_subdirs;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};

use crate::git::WatchedRepository;

// TODO: configurable delay (config)
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
    Watch(Watch),
}

#[derive(Args)]
struct Watch {
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

impl Watch {
    fn run(&mut self) {
        let (tx, rx): (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(self.delay)).unwrap();

        let directories = list_subdirs(
            &self.directory,
            HashSet::from_iter(vec![".obsidian", ".git"].into_iter().map(|s| OsStr::new(s))),
        );

        println!("{:#?}", directories);

        for dir in directories {
            watcher.watch(dir, RecursiveMode::NonRecursive).unwrap();
        }

        let repo = WatchedRepository::new(&self.directory).unwrap();

        loop {
            match rx.recv() {
                Ok(event) => self.handle_event(&event, &repo),
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }

    fn handle_event(&self, event: &DebouncedEvent, repo: &WatchedRepository) {
        println!("{:?}", event);
        // TODO: better commit messages (e.g. short title, descriptive body)
        // TODO: configurable commit messages
        let (path, message) = match event {
            DebouncedEvent::Create(path) => (
                path,
                format!(
                    "created file {} @ {}",
                    path.to_str().unwrap(),
                    chrono::Utc::now()
                ),
            ),
            DebouncedEvent::Write(path) => (
                path,
                format!(
                    "written file {} @ {}",
                    path.to_str().unwrap(),
                    chrono::Utc::now()
                ),
            ),
            DebouncedEvent::Chmod(path) => (
                path,
                format!(
                    "chmod file {} @ {}",
                    path.to_str().unwrap(),
                    chrono::Utc::now()
                ),
            ),
            DebouncedEvent::Remove(path) => (
                path,
                format!(
                    "deleted file {} @ {}",
                    path.to_str().unwrap(),
                    chrono::Utc::now()
                ),
            ),
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
    let mut cli = Cli::parse();
    match &mut cli.commands {
        Commands::Watch(watch) => watch.run(),
    }
}
