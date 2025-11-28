use crate::model::Task;
use anyhow::Result;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

// Constants for identification
pub const LOCAL_CALENDAR_HREF: &str = "local://default";
pub const LOCAL_CALENDAR_NAME: &str = "Local";

pub struct LocalStorage;

impl LocalStorage {
    fn get_path() -> Option<PathBuf> {
        if let Some(proj) = ProjectDirs::from("com", "trougnouf", "cfait") {
            let data_dir = proj.data_dir();
            if !data_dir.exists() {
                let _ = fs::create_dir_all(data_dir);
            }
            return Some(data_dir.join("local.json"));
        }
        None
    }

    pub fn save(tasks: &[Task]) -> Result<()> {
        if let Some(path) = Self::get_path() {
            let json = serde_json::to_string_pretty(tasks)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    pub fn load() -> Result<Vec<Task>> {
        if let Some(path) = Self::get_path()
            && path.exists()
        {
            // If the file exists but is empty/corrupt, ignore error and return empty vec
            if let Ok(json) = fs::read_to_string(path) {
                if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(&json) {
                    return Ok(tasks);
                }
            }
        }
        Ok(vec![])
    }
}
