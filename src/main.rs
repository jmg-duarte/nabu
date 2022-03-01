
use clap::{Parser, Subcommand};

pub fn default_config_path() -> String {
    let home = std::env::var("HOME").unwrap();
    return home + "/.config/seshat.toml";
}

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Watch {
        /// The directory to watch over.
        directory: String,
        /// Whether to watch sub-directories.
        #[clap(short, long)]
        recursive: bool,
        /// Path to the configuration file.
        #[clap(short, long, default_value_t=default_config_path())]
        config: String,
    }
}

fn main() {
    let cli = Cli::parse();
}
