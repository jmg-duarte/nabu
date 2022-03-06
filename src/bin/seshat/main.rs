mod watch;

use flexi_logger::Logger;
use seshat::config::{global_config_path, local_config_path, Config};

use watch::{Watch, WatchArgs};

use clap::{Parser, Subcommand};
use color_eyre::Result;

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    commands: Commands,
    /// Path to the configuration file.
    #[clap(short, long)]
    config: Option<String>,
    /// Print debug information.
    #[clap(long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch over a given directory
    Watch(WatchArgs),
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    if cli.debug {
        Logger::try_with_str("debug")
    } else {
        Logger::try_with_str("info")
    }?
    .start()?;

    let config = Config::from_path(local_config_path())
        .or_else(|_| Config::from_path(global_config_path()))
        .unwrap_or(Config::default());

    match cli.commands {
        Commands::Watch(args) => Watch::from(args).run(config),
    }

    Ok(())
}
