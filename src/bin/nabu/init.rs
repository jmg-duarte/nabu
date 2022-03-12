use std::env::current_dir;

use clap::Args;
use log::info;
use nabu::config::{global_config_path, Config};

#[derive(Args)]
pub(crate) struct InitArgs {
    /// Global configuration file
    #[clap(long)]
    global: bool,
}

impl InitArgs {
    pub fn run(self) {
        let config = Config::default();
        let config_toml = toml::to_string_pretty(&config).unwrap();
        let path = if self.global {
            global_config_path()
        } else {
            let mut local_config_path = current_dir().unwrap();
            local_config_path.push("nabu.toml");
            local_config_path
        };
        std::fs::write(&path, config_toml).unwrap();
        info!("config file written to {}", path.display());
    }
}
