// File: src/mobile.rs
use crate::client::RustyClient;
use crate::config::Config;
use crate::model::Task;
use crate::paths::AppPaths;
use crate::storage::{LOCAL_CALENDAR_HREF, LocalStorage};
#[cfg(target_os = "android")]
use android_logger::Config as LogConfig;
#[cfg(target_os = "android")]
use log::LevelFilter;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum MobileError {
    Generic(String),
}
impl From<String> for MobileError {
    fn from(e: String) -> Self {
        Self::Generic(e)
    }
}
impl From<&str> for MobileError {
    fn from(e: &str) -> Self {
        Self::Generic(e.to_string())
    }
}
impl From<anyhow::Error> for MobileError {
    fn from(e: anyhow::Error) -> Self {
        Self::Generic(e.to_string())
    }
}
impl std::fmt::Display for MobileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MobileError::Generic(s) => s,
            }
        )
    }
}
impl std::error::Error for MobileError {}

#[derive(uniffi::Record)]
pub struct MobileTask {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub is_done: bool,
    pub priority: u8,
    pub due_date_iso: Option<String>,
    pub calendar_href: String,
    pub categories: Vec<String>,
}

impl From<Task> for MobileTask {
    fn from(t: Task) -> Self {
        Self {
            uid: t.uid,
            summary: t.summary,
            description: t.description,
            is_done: t.status.is_done(),
            priority: t.priority,
            due_date_iso: t.due.map(|d| d.to_rfc3339()),
            calendar_href: t.calendar_href,
            categories: t.categories,
        }
    }
}

// DTO for Settings
#[derive(uniffi::Record)]
pub struct MobileConfig {
    pub url: String,
    pub username: String,
    pub default_calendar: Option<String>,
    pub allow_insecure: bool,
    pub hide_completed: bool,
}

#[derive(uniffi::Object)]
pub struct CfaitMobile {
    client: Arc<Mutex<Option<RustyClient>>>,
}

// --- EXPORTED METHODS (Called from Kotlin) ---
#[uniffi::export(async_runtime = "tokio")]
impl CfaitMobile {
    #[uniffi::constructor]
    pub fn new(android_files_dir: String) -> Self {
        #[cfg(target_os = "android")]
        android_logger::init_once(
            LogConfig::default()
                .with_max_level(LevelFilter::Debug)
                .with_tag("CfaitRust"),
        );
        AppPaths::init_android_path(android_files_dir);
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }

    /// Read config for UI population
    pub fn get_config(&self) -> MobileConfig {
        let c = Config::load().unwrap_or_default();
        MobileConfig {
            url: c.url,
            username: c.username,
            default_calendar: c.default_calendar,
            allow_insecure: c.allow_insecure_certs,
            hide_completed: c.hide_completed,
        }
    }

    /// Save config from UI
    pub fn save_config(
        &self,
        url: String,
        user: String,
        pass: String,
        insecure: bool,
        hide_completed: bool,
    ) -> Result<(), MobileError> {
        let mut c = Config::load().unwrap_or_default();
        c.url = url;
        c.username = user;
        if !pass.is_empty() {
            c.password = pass;
        }
        c.allow_insecure_certs = insecure;
        c.hide_completed = hide_completed;
        c.save().map_err(MobileError::from)
    }

    pub async fn load_and_connect(&self) -> Result<String, MobileError> {
        let config = Config::load().map_err(MobileError::from)?;
        self.connect_internal(config).await
    }

    /// Connect explicitly (e.g. from Settings)
    pub async fn connect(
        &self,
        url: String,
        user: String,
        pass: String,
        insecure: bool,
    ) -> Result<String, MobileError> {
        let mut config = Config::load().unwrap_or_default();
        config.url = url;
        config.username = user;
        if !pass.is_empty() {
            config.password = pass;
        }
        config.allow_insecure_certs = insecure;

        let res = self.connect_internal(config.clone()).await;
        if res.is_ok() {
            let _ = config.save();
        }
        res
    }

    pub async fn get_tasks(&self) -> Vec<MobileTask> {
        let mut tasks = Vec::new();
        if let Ok(local) = LocalStorage::load() {
            tasks.extend(local);
        }

        if let Ok(cals) = crate::cache::Cache::load_calendars() {
            for cal in cals {
                if cal.href == LOCAL_CALENDAR_HREF {
                    continue;
                }
                if let Ok((cached, _)) = crate::cache::Cache::load(&cal.href) {
                    tasks.extend(cached);
                }
            }
        }
        tasks.into_iter().map(MobileTask::from).collect()
    }

    pub async fn add_task_smart(&self, input: String) -> Result<(), MobileError> {
        let aliases = std::collections::HashMap::new();
        let mut task = Task::new(&input, &aliases);

        let guard = self.client.lock().await;
        if let Some(client) = &*guard {
            let target_href = if let Ok(cfg) = Config::load() {
                cfg.default_calendar
                    .unwrap_or(LOCAL_CALENDAR_HREF.to_string())
            } else {
                LOCAL_CALENDAR_HREF.to_string()
            };
            task.calendar_href = target_href;
            client
                .create_task(&mut task)
                .await
                .map(|_| ())
                .map_err(MobileError::from)
        } else {
            task.calendar_href = LOCAL_CALENDAR_HREF.to_string();
            let mut all = LocalStorage::load().unwrap_or_default();
            all.push(task);
            LocalStorage::save(&all).map_err(MobileError::from)
        }
    }

    pub async fn toggle_task(&self, uid: String) -> Result<(), MobileError> {
        self.modify_task(uid, |t| {
            if t.status.is_done() {
                t.status = crate::model::TaskStatus::NeedsAction;
            } else {
                t.status = crate::model::TaskStatus::Completed;
            }
        })
        .await
    }

    pub async fn delete_task(&self, uid: String) -> Result<(), MobileError> {
        if let Ok(mut local) = LocalStorage::load() {
            if let Some(pos) = local.iter().position(|t| t.uid == uid) {
                local.remove(pos);
                return LocalStorage::save(&local).map_err(MobileError::from);
            }
        }
        let guard = self.client.lock().await;
        if let Some(client) = &*guard {
            if let Some((task, _)) = self.find_task_in_cache(&uid) {
                client
                    .delete_task(&task)
                    .await
                    .map(|_| ())
                    .map_err(MobileError::from)?;
                return Ok(());
            }
        }
        Err(MobileError::from("Task not found"))
    }
}

// --- INTERNAL HELPERS (Not exposed to Kotlin) ---
impl CfaitMobile {
    async fn connect_internal(&self, config: Config) -> Result<String, MobileError> {
        match RustyClient::connect_with_fallback(config).await {
            Ok((client, calendars, _, _, warning)) => {
                *self.client.lock().await = Some(client.clone());

                // Force sync all calendars to populate cache ---
                // This ensures get_tasks() returns remote items immediately.
                if let Err(e) = client.get_all_tasks(&calendars).await {
                    println!("Initial sync failed: {:?}", e); // Non-fatal, just log
                }

                Ok(warning.unwrap_or_else(|| "Connected".to_string()))
            }
            Err(e) => Err(MobileError::from(e)),
        }
    }

    async fn modify_task<F>(&self, uid: String, mut modifier: F) -> Result<(), MobileError>
    where
        F: FnMut(&mut Task),
    {
        if let Ok(mut local) = LocalStorage::load() {
            if let Some(pos) = local.iter().position(|t| t.uid == uid) {
                modifier(&mut local[pos]);
                return LocalStorage::save(&local).map_err(MobileError::from);
            }
        }
        let guard = self.client.lock().await;
        if let Some(client) = &*guard {
            if let Some((mut task, _)) = self.find_task_in_cache(&uid) {
                modifier(&mut task);
                let _ = client
                    .update_task(&mut task)
                    .await
                    .map_err(MobileError::from)?;
                return Ok(());
            }
        }
        Err(MobileError::from("Task not found"))
    }

    fn find_task_in_cache(&self, uid: &str) -> Option<(Task, String)> {
        if let Ok(cals) = crate::cache::Cache::load_calendars() {
            for cal in cals {
                if let Ok((tasks, _)) = crate::cache::Cache::load(&cal.href) {
                    if let Some(t) = tasks.into_iter().find(|t| t.uid == uid) {
                        return Some((t, cal.href));
                    }
                }
            }
        }
        None
    }
}
