use anyhow::Result;
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub url: String,
    pub username: String,
    pub password: String,
    // New Optional Field
    pub default_calendar: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "trougnouf", "rustache") {
            let config_path = proj_dirs.config_dir().join("config.toml");

            if config_path.exists() {
                let contents = fs::read_to_string(config_path)?;
                let config: Config = toml::from_str(&contents)?;
                return Ok(config);
            }
        }

        Err(anyhow::anyhow!("Config file not found"))
    }
}
