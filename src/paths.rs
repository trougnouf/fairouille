// File: src/paths.rs
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

// Allow injecting a base path (from Android Context)
static ANDROID_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub struct AppPaths;

impl AppPaths {
    /// Initialize the Android data directory. Must be called once at startup.
    pub fn init_android_path(path: String) {
        let _ = ANDROID_DATA_DIR.set(PathBuf::from(path));
    }

    /// Returns the ProjectDirs struct, common to all path lookups.
    fn get_proj_dirs() -> Option<ProjectDirs> {
        ProjectDirs::from("com", "cfait", "cfait")
            .or_else(|| ProjectDirs::from("com", "trougnouf", "cfait"))
    }

    /// Helper to ensure a directory exists before returning it.
    fn ensure_exists(path: PathBuf) -> Result<PathBuf> {
        if !path.exists() {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create directory: {:?}", path))?;
        }
        Ok(path)
    }

    /// Determines the logic for the base directory based on environment variables or OS defaults.
    fn resolve_base(subdir: &str) -> Option<PathBuf> {
        // 1. Android Override
        if let Some(android_root) = ANDROID_DATA_DIR.get() {
            // On Android, everything goes into the app's private files directory.
            return Some(android_root.join(subdir));
        }

        // 2. Test Override
        if let Ok(test_dir) = env::var("CFAIT_TEST_DIR") {
            return Some(PathBuf::from(test_dir));
        }

        // 3. Standard OS location
        let proj = Self::get_proj_dirs()?;

        let dir = match subdir {
            "data" => proj.data_dir(),
            "config" => proj.config_dir(),
            "cache" => proj.cache_dir(),
            _ => return None,
        };

        Some(dir.to_path_buf())
    }

    pub fn get_data_dir() -> Result<PathBuf> {
        let path = Self::resolve_base("data")
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
        Self::ensure_exists(path)
    }

    pub fn get_config_dir() -> Result<PathBuf> {
        let path = Self::resolve_base("config")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Self::ensure_exists(path)
    }

    pub fn get_cache_dir() -> Result<PathBuf> {
        let path = Self::resolve_base("cache")
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;
        Self::ensure_exists(path)
    }

    pub fn get_config_file_path() -> Result<PathBuf> {
        Ok(Self::get_config_dir()?.join("config.toml"))
    }

    pub fn get_journal_path() -> Option<PathBuf> {
        Self::get_data_dir().ok().map(|p| p.join("journal.json"))
    }

    pub fn get_local_task_path() -> Option<PathBuf> {
        Self::get_data_dir().ok().map(|p| p.join("local.json"))
    }
}
