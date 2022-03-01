mod fs;

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

const DEFAULT_DELAY: u64 = 1;

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
}

impl Watch {
    fn run(&mut self) {
        let (tx, rx): (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(DEFAULT_DELAY)).unwrap();

        let directories = list_subdirs(
            &self.directory,
            HashSet::from_iter(vec![".obsidian", ".git"].into_iter().map(|s| OsStr::new(s))),
        );

        println!("{:#?}", directories);

        for dir in directories {
            watcher.watch(dir, RecursiveMode::NonRecursive).unwrap();
        }

        loop {
            match rx.recv() {
                Ok(event) => println!("{:?}", event),
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }
}


fn main() {
    let mut cli = Cli::parse();
    match &mut cli.commands {
        Commands::Watch(watch) => watch.run(),
    }
}
