mod init;
mod watch;

use flexi_logger::Logger;
use init::InitArgs;

use watch::{Watch, WatchArgs};

use clap::{Parser, Subcommand};
use color_eyre::Result;

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    commands: Commands,
    /// Print debug information.
    #[clap(long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a `nabu.toml` configuration file.
    Init(InitArgs),
    /// Watch over a given directory
    Watch(WatchArgs),
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let level = if cli.debug { "debug" } else { "info" };
    Logger::try_with_str(level)?.start()?;

    match cli.commands {
        Commands::Watch(args) => Watch::from(args).run(),
        Commands::Init(init) => init.run(),
    }

    Ok(())
}
