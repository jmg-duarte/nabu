mod init;
mod watch;

use flexi_logger::Logger;
use init::InitArgs;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use watch::WatchArgs;

use clap::{Parser, Subcommand};
use color_eyre::Result;

use ctrlc;

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

    let watching = Arc::new(AtomicBool::new(true));
    let r = watching.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let level = if cli.debug { "debug" } else { "info" };
    Logger::try_with_str(level)?.use_utc().start()?;

    match cli.commands {
        Commands::Watch(args) => {
            args.run(watching)?;
        }
        Commands::Init(init) => init.run(),
    }

    Ok(())
}
