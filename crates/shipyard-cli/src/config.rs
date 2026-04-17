use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) api_url: Option<String>,
    pub(crate) session_token: Option<String>,
    pub(crate) default_account: Option<String>,
    pub(crate) output: Option<String>,
}

impl Config {
    pub(crate) fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path).context("failed to read config")?;
        toml::from_str(&contents).context("failed to parse config")
    }

    pub(crate) fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("failed to create config directory")?;
        }
        let contents = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(path, contents).context("failed to write config")
    }
}

pub(crate) fn config_path(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = path {
        return Ok(path);
    }
    let base = dirs::config_dir().context("could not find config directory")?;
    Ok(base.join("shipyard").join("config.toml"))
}
