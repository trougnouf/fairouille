use crate::model::Task;
use anyhow::Result;
use directories::ProjectDirs;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub struct Cache;

impl Cache {
    // Generate a filename based on the Calendar URL (or "default")
    fn get_path(key: &str) -> Option<PathBuf> {
        if let Some(proj) = ProjectDirs::from("com", "rustache", "rustache") {
            let cache_dir = proj.cache_dir();
            if !cache_dir.exists() {
                let _ = fs::create_dir_all(cache_dir);
            }

            // Hash the URL to get a safe, unique filename
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            let filename = format!("tasks_{:x}.json", hasher.finish());

            return Some(cache_dir.join(filename));
        }
        None
    }

    pub fn save(key: &str, tasks: &[Task]) -> Result<()> {
        if let Some(path) = Self::get_path(key) {
            let json = serde_json::to_string_pretty(tasks)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    pub fn load(key: &str) -> Result<Vec<Task>> {
        if let Some(path) = Self::get_path(key) {
            if path.exists() {
                let json = fs::read_to_string(path)?;
                let tasks: Vec<Task> = serde_json::from_str(&json)?;
                return Ok(tasks);
            }
        }
        Ok(vec![])
    }
}
