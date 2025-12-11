// File: src/storage.rs
use crate::model::Task;
use crate::paths::AppPaths;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "android"))]
use fs2::FileExt;

// Constants for identification
pub const LOCAL_CALENDAR_HREF: &str = "local://default";
pub const LOCAL_CALENDAR_NAME: &str = "Local";

pub struct LocalStorage;

impl LocalStorage {
    pub fn get_path() -> Option<PathBuf> {
        AppPaths::get_local_task_path()
    }

    /// Helper to get a sidecar lock file path (e.g., "local.json.lock")
    #[cfg(not(target_os = "android"))]
    fn get_lock_path(file_path: &Path) -> PathBuf {
        let mut lock_path = file_path.to_path_buf();
        if let Some(ext) = lock_path.extension() {
            let mut new_ext = ext.to_os_string();
            new_ext.push(".lock");
            lock_path.set_extension(new_ext);
        } else {
            lock_path.set_extension("lock");
        }
        lock_path
    }

    /// Execute a closure while holding an exclusive lock on the sidecar file.
    pub fn with_lock<F, T>(file_path: &Path, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        #[cfg(target_os = "android")]
        {
            // Silence the warning explicitly for Android
            let _ = file_path;
            f()
        }

        #[cfg(not(target_os = "android"))]
        {
            let lock_path = Self::get_lock_path(file_path); // Now this works
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path)?;

            file.lock_exclusive()?;
            let result = f();
            file.unlock()?;
            result
        }
    }

    /// Atomic write: Write to .tmp file then rename
    pub fn atomic_write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
        let path = path.as_ref();
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, contents)?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    pub fn save(tasks: &[Task]) -> Result<()> {
        if let Some(path) = Self::get_path() {
            Self::with_lock(&path, || {
                let json = serde_json::to_string_pretty(tasks)?;
                Self::atomic_write(&path, json)?;
                Ok(())
            })?;
        }
        Ok(())
    }

    pub fn load() -> Result<Vec<Task>> {
        if let Some(path) = Self::get_path() {
            if !path.exists() {
                return Ok(vec![]);
            }
            return Self::with_lock(&path, || {
                let json = fs::read_to_string(&path)?;
                // CHANGE: Propagate error instead of checking `if let Ok`
                let tasks = serde_json::from_str::<Vec<Task>>(&json)?;
                Ok(tasks)
            });
        }
        Ok(vec![])
    }
}
