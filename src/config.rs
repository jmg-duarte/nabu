use std::{
    env::current_dir,
    path::{Path, PathBuf},
};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

/// Default watcher delay (in seconds).
const DEFAULT_DELAY: u64 = 30;

fn default_delay() -> u64 {
    DEFAULT_DELAY
}

pub fn global_config_path() -> PathBuf {
    let path = std::env::var("HOME").unwrap() + "/.config/nabu.toml";
    PathBuf::from(path)
}

pub fn local_config_path() -> PathBuf {
    let mut path_buf = current_dir().unwrap();
    path_buf.push("nabu.toml");
    path_buf
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_delay")]
    pub delay: u64,
    #[serde(default = "Vec::new")]
    pub ignore: Vec<String>,
}

impl Config {
    pub fn from_path<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let bytes = std::fs::read(path)?;
        Ok(toml::from_slice::<Config>(bytes.as_slice())?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delay: DEFAULT_DELAY,
            ignore: vec![],
        }
    }
}
