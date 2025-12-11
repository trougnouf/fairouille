// File: src/config.rs
use crate::paths::AppPaths;
use crate::storage::LocalStorage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

fn default_true() -> bool {
    true
}
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
    pub disabled_calendars: Vec<String>,
    #[serde(default)]
    pub hide_completed: bool,
    #[serde(default = "default_true")]
    pub hide_fully_completed_tags: bool,
    #[serde(default = "default_cutoff")]
    pub sort_cutoff_months: Option<u32>,
    #[serde(default)]
    pub tag_aliases: HashMap<String, Vec<String>>,
}

// --- ADDED THIS IMPLEMENTATION ---
impl Default for Config {
    fn default() -> Self {
        Self {
            url: String::new(),
            username: String::new(),
            password: String::new(),
            default_calendar: None,
            allow_insecure_certs: false,
            hidden_calendars: Vec::new(),
            disabled_calendars: Vec::new(),
            hide_completed: false,
            // Match the serde defaults
            hide_fully_completed_tags: true,
            sort_cutoff_months: Some(6),
            tag_aliases: HashMap::new(),
        }
    }
}
// --------------------------------

impl Config {
    // ... keep existing implementation ...
    pub fn load() -> Result<Self> {
        let path = AppPaths::get_config_file_path()?;
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok(config);
        }
        Err(anyhow::anyhow!("Config file not found"))
    }

    pub fn save(&self) -> Result<()> {
        let path = AppPaths::get_config_file_path()?;
        LocalStorage::with_lock(&path, || {
            let toml_str = toml::to_string_pretty(self)?;
            LocalStorage::atomic_write(&path, toml_str)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn get_path_string() -> Result<String> {
        let path = AppPaths::get_config_file_path()?;
        Ok(path.to_string_lossy().to_string())
    }
}
