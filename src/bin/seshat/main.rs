mod watch;

use watch::{Watch, WatchArgs};

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch over a given directory
    Watch(WatchArgs),
}

fn main() {
    let cli = Cli::parse();
    match cli.commands {
        Commands::Watch(args) => Watch::from(args).run(),
    }
}
