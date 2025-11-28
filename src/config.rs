use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

// Default to 6 months
fn default_cutoff() -> Option<u32> {
    Some(6)
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub url: String,
    pub username: String,
    pub password: String,
    pub default_calendar: Option<String>,

    #[serde(default)]
    pub allow_insecure_certs: bool,

    #[serde(default)]
    pub hidden_calendars: Vec<String>,

    #[serde(default)]
    pub hide_completed: bool,
    #[serde(default = "default_true")]
    pub hide_fully_completed_tags: bool,

    #[serde(default = "default_cutoff")]
    pub sort_cutoff_months: Option<u32>,

    #[serde(default)]
    pub tag_aliases: HashMap<String, Vec<String>>,
}

impl Config {
    fn get_path() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "trougnouf", "cfait") {
            let config_dir = proj_dirs.config_dir();
            if !config_dir.exists() {
                fs::create_dir_all(config_dir)?;
            }
            return Ok(config_dir.join("config.toml"));
        }
        Err(anyhow::anyhow!("Could not determine config path"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok(config);
        }
        Err(anyhow::anyhow!("Config file not found"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(path, toml_str)?;
        Ok(())
    }
    pub fn get_path_string() -> Result<String> {
        let path = Self::get_path()?;
        Ok(path.to_string_lossy().to_string())
    }
}
