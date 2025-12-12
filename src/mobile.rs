// File: ./src/mobile.rs
use crate::cache::Cache;
use crate::client::RustyClient;
use crate::config::Config;
use crate::model::Task;
use crate::paths::AppPaths;
use crate::storage::{LOCAL_CALENDAR_HREF, LOCAL_CALENDAR_NAME, LocalStorage};
use crate::store::{FilterOptions, TaskStore, UNCATEGORIZED_ID};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(target_os = "android")]
use android_logger::Config as LogConfig;
#[cfg(target_os = "android")]
use log::LevelFilter;

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

// --- DTOs ---

#[derive(uniffi::Record)]
pub struct MobileTask {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub is_done: bool,
    pub priority: u8,
    pub due_date_iso: Option<String>,
    pub start_date_iso: Option<String>,
    pub duration_mins: Option<u32>,
    pub calendar_href: String,
    pub categories: Vec<String>,
    pub is_recurring: bool,
    pub parent_uid: Option<String>,
    pub smart_string: String,
    pub depth: u32,
    pub is_blocked: bool,
    pub status_string: String,
    pub blocked_by_names: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct MobileCalendar {
    pub name: String,
    pub href: String,
    pub color: Option<String>,
    pub is_visible: bool,
    pub is_local: bool,
    pub is_disabled: bool,
}

#[derive(uniffi::Record)]
pub struct MobileTag {
    pub name: String,
    pub count: u32,
    pub is_uncategorized: bool,
}

#[derive(uniffi::Record)]
pub struct MobileConfig {
    pub url: String,
    pub username: String,
    pub default_calendar: Option<String>,
    pub allow_insecure: bool,
    pub hide_completed: bool,
    pub tag_aliases: HashMap<String, Vec<String>>,
    pub disabled_calendars: Vec<String>,
}

fn task_to_mobile(t: &Task, store: &TaskStore) -> MobileTask {
    let smart = t.to_smart_string();
    let status_str = format!("{:?}", t.status);
    let is_blocked = store.is_blocked(t);
    let blocked_by_names = t
        .dependencies
        .iter()
        .filter_map(|uid| store.get_summary(uid))
        .collect();
    MobileTask {
        uid: t.uid.clone(),
        summary: t.summary.clone(),
        description: t.description.clone(),
        is_done: t.status.is_done(),
        priority: t.priority,
        due_date_iso: t.due.map(|d| d.to_rfc3339()),
        start_date_iso: t.dtstart.map(|d| d.to_rfc3339()),
        duration_mins: t.estimated_duration,
        calendar_href: t.calendar_href.clone(),
        categories: t.categories.clone(),
        is_recurring: t.rrule.is_some(),
        parent_uid: t.parent_uid.clone(),
        smart_string: smart,
        depth: t.depth as u32,
        is_blocked,
        status_string: status_str,
        blocked_by_names,
    }
}

// --- MAIN OBJECT ---

#[derive(uniffi::Object)]
pub struct CfaitMobile {
    client: Arc<Mutex<Option<RustyClient>>>,
    store: Arc<Mutex<TaskStore>>,
}

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
            store: Arc::new(Mutex::new(TaskStore::new())),
        }
    }

    pub fn get_config(&self) -> MobileConfig {
        let c = Config::load().unwrap_or_default();
        MobileConfig {
            url: c.url,
            username: c.username,
            default_calendar: c.default_calendar,
            allow_insecure: c.allow_insecure_certs,
            hide_completed: c.hide_completed,
            tag_aliases: c.tag_aliases,
            disabled_calendars: c.disabled_calendars,
        }
    }

    pub fn save_config(
        &self,
        url: String,
        user: String,
        pass: String,
        insecure: bool,
        hide_completed: bool,
        disabled_calendars: Vec<String>,
    ) -> Result<(), MobileError> {
        let mut c = Config::load().unwrap_or_default();
        c.url = url;
        c.username = user;
        if !pass.is_empty() {
            c.password = pass;
        }
        c.allow_insecure_certs = insecure;
        c.hide_completed = hide_completed;
        c.disabled_calendars = disabled_calendars;
        c.save().map_err(MobileError::from)
    }

    pub fn add_alias(&self, key: String, tags: Vec<String>) -> Result<(), MobileError> {
        let mut c = Config::load().unwrap_or_default();
        c.tag_aliases.insert(key, tags);
        c.save().map_err(MobileError::from)
    }
    pub fn remove_alias(&self, key: String) -> Result<(), MobileError> {
        let mut c = Config::load().unwrap_or_default();
        c.tag_aliases.remove(&key);
        c.save().map_err(MobileError::from)
    }
    pub fn set_default_calendar(&self, href: String) -> Result<(), MobileError> {
        let mut config = Config::load().map_err(MobileError::from)?;
        config.default_calendar = Some(href);
        config.save().map_err(MobileError::from)
    }
    pub fn set_calendar_visibility(&self, href: String, visible: bool) -> Result<(), MobileError> {
        let mut config = Config::load().map_err(MobileError::from)?;
        if visible {
            config.hidden_calendars.retain(|h| h != &href);
        } else if !config.hidden_calendars.contains(&href) {
            config.hidden_calendars.push(href);
        }
        config.save().map_err(MobileError::from)
    }
    pub fn load_from_cache(&self) {
        let mut store = self.store.blocking_lock();
        store.clear();
        if let Ok(local) = LocalStorage::load() {
            store.insert(LOCAL_CALENDAR_HREF.to_string(), local);
        }
        if let Ok(cals) = Cache::load_calendars() {
            for cal in cals {
                if cal.href == LOCAL_CALENDAR_HREF {
                    continue;
                }
                if let Ok((tasks, _)) = Cache::load(&cal.href) {
                    store.insert(cal.href, tasks);
                }
            }
        }
    }
    pub async fn sync(&self) -> Result<String, MobileError> {
        let config = Config::load().map_err(MobileError::from)?;
        self.apply_connection(config).await
    }
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
        self.apply_connection(config).await
    }

    pub fn get_calendars(&self) -> Vec<MobileCalendar> {
        let config = Config::load().unwrap_or_default();
        let disabled_set: HashSet<String> = config.disabled_calendars.iter().cloned().collect();
        let mut result = Vec::new();

        let local_href = LOCAL_CALENDAR_HREF.to_string();
        result.push(MobileCalendar {
            name: LOCAL_CALENDAR_NAME.to_string(),
            href: local_href.clone(),
            color: None,
            is_visible: !config.hidden_calendars.contains(&local_href),
            is_local: true,
            is_disabled: false,
        });

        if let Ok(cals) = crate::cache::Cache::load_calendars() {
            for c in cals {
                if c.href == LOCAL_CALENDAR_HREF {
                    continue;
                }
                result.push(MobileCalendar {
                    name: c.name,
                    href: c.href.clone(),
                    color: c.color,
                    is_visible: !config.hidden_calendars.contains(&c.href),
                    is_local: false,
                    is_disabled: disabled_set.contains(&c.href),
                });
            }
        }
        result
    }

    pub async fn get_all_tags(&self) -> Vec<MobileTag> {
        let store = self.store.lock().await;
        let config = Config::load().unwrap_or_default();
        let empty_includes = HashSet::new();
        let mut hidden_cals: HashSet<String> = config.hidden_calendars.into_iter().collect();
        hidden_cals.extend(config.disabled_calendars);

        store
            .get_all_categories(
                config.hide_completed,
                config.hide_fully_completed_tags,
                &empty_includes,
                &hidden_cals,
            )
            .into_iter()
            .map(|(name, count)| MobileTag {
                name: name.clone(),
                count: count as u32,
                is_uncategorized: name == UNCATEGORIZED_ID,
            })
            .collect()
    }

    pub async fn get_view_tasks(
        &self,
        filter_tag: Option<String>,
        search_query: String,
    ) -> Vec<MobileTask> {
        let store = self.store.lock().await;
        let config = Config::load().unwrap_or_default();
        let mut selected_categories = HashSet::new();
        if let Some(tag) = filter_tag {
            selected_categories.insert(tag);
        }

        let mut hidden: HashSet<String> = config.hidden_calendars.into_iter().collect();
        hidden.extend(config.disabled_calendars);

        let cutoff_date = if let Some(months) = config.sort_cutoff_months {
            Some(chrono::Utc::now() + chrono::Duration::days(months as i64 * 30))
        } else {
            None
        };

        let filtered = store.filter(FilterOptions {
            active_cal_href: None,
            hidden_calendars: &hidden,
            selected_categories: &selected_categories,
            match_all_categories: false,
            search_term: &search_query,
            hide_completed_global: config.hide_completed,
            cutoff_date,
            min_duration: None,
            max_duration: None,
            include_unset_duration: true,
        });
        filtered
            .into_iter()
            .map(|t| task_to_mobile(&t, &store))
            .collect()
    }

    pub async fn yank_task(&self, _uid: String) -> Result<(), MobileError> {
        Ok(())
    }

    pub async fn add_task_smart(&self, input: String) -> Result<(), MobileError> {
        let aliases = Config::load().unwrap_or_default().tag_aliases;
        let mut task = Task::new(&input, &aliases);
        let guard = self.client.lock().await;
        let config = Config::load().unwrap_or_default();
        let target_href = config
            .default_calendar
            .clone()
            .unwrap_or(LOCAL_CALENDAR_HREF.to_string());
        task.calendar_href = target_href.clone();
        if let Some(client) = &*guard {
            client
                .create_task(&mut task)
                .await
                .map(|_| ())
                .map_err(MobileError::from)?;
        } else {
            let mut all = LocalStorage::load().unwrap_or_default();
            all.push(task.clone());
            LocalStorage::save(&all).map_err(MobileError::from)?;
        }
        self.store.lock().await.add_task(task);
        Ok(())
    }
    pub async fn change_priority(&self, uid: String, delta: i8) -> Result<(), MobileError> {
        self.modify_task_and_sync(uid, |t| {
            t.priority = if delta > 0 {
                match t.priority {
                    0 => 9,
                    9 => 5,
                    5 => 1,
                    1 => 1,
                    _ => 5,
                }
            } else {
                match t.priority {
                    1 => 5,
                    5 => 9,
                    9 => 0,
                    0 => 0,
                    _ => 0,
                }
            };
        })
        .await
    }
    pub async fn set_status_process(&self, uid: String) -> Result<(), MobileError> {
        self.modify_task_and_sync(uid, |t| {
            t.status = if t.status == crate::model::TaskStatus::InProcess {
                crate::model::TaskStatus::NeedsAction
            } else {
                crate::model::TaskStatus::InProcess
            };
        })
        .await
    }
    pub async fn set_status_cancelled(&self, uid: String) -> Result<(), MobileError> {
        self.modify_task_and_sync(uid, |t| {
            t.status = if t.status == crate::model::TaskStatus::Cancelled {
                crate::model::TaskStatus::NeedsAction
            } else {
                crate::model::TaskStatus::Cancelled
            };
        })
        .await
    }
    pub async fn update_task_smart(
        &self,
        uid: String,
        smart_input: String,
    ) -> Result<(), MobileError> {
        let aliases = Config::load().unwrap_or_default().tag_aliases;
        self.modify_task_and_sync(uid, |t| {
            t.apply_smart_input(&smart_input, &aliases);
        })
        .await
    }
    pub async fn update_task_description(
        &self,
        uid: String,
        description: String,
    ) -> Result<(), MobileError> {
        self.modify_task_and_sync(uid, |t| {
            t.description = description.clone();
        })
        .await
    }
    pub async fn toggle_task(&self, uid: String) -> Result<(), MobileError> {
        self.modify_task_and_sync(uid, |t| {
            if t.status.is_done() {
                t.status = crate::model::TaskStatus::NeedsAction;
            } else {
                t.status = crate::model::TaskStatus::Completed;
            }
        })
        .await
    }
    pub async fn move_task(&self, uid: String, new_cal_href: String) -> Result<(), MobileError> {
        let mut store = self.store.lock().await;
        let updated_task = store
            .move_task(&uid, new_cal_href.clone())
            .ok_or(MobileError::from("Task not found"))?;
        let client_guard = self.client.lock().await;
        if let Some(client) = &*client_guard {
            client
                .move_task(&updated_task, &new_cal_href)
                .await
                .map_err(MobileError::from)?;
        } else {
            return Err(MobileError::from("Client offline"));
        }
        Ok(())
    }
    pub async fn delete_task(&self, uid: String) -> Result<(), MobileError> {
        let mut store = self.store.lock().await;
        let task = store
            .delete_task(&uid)
            .ok_or(MobileError::from("Task not found"))?;
        let client_guard = self.client.lock().await;
        if let Some(client) = &*client_guard {
            client.delete_task(&task).await.map_err(MobileError::from)?;
        } else if task.calendar_href == LOCAL_CALENDAR_HREF {
            let mut local = LocalStorage::load().unwrap_or_default();
            if let Some(pos) = local.iter().position(|t| t.uid == uid) {
                local.remove(pos);
                LocalStorage::save(&local).map_err(MobileError::from)?;
            }
        }
        Ok(())
    }
}

impl CfaitMobile {
    async fn apply_connection(&self, config: Config) -> Result<String, MobileError> {
        // 1. Initial Connect & Calendar Discovery
        let (client, cals, _initial_tasks, _active, warning) =
            RustyClient::connect_with_fallback(config)
                .await
                .map_err(MobileError::from)?;

        *self.client.lock().await = Some(client.clone());
        let mut store = self.store.lock().await;
        store.clear();

        // 2. Load Local Calendar
        if let Ok(local) = LocalStorage::load() {
            store.insert(LOCAL_CALENDAR_HREF.to_string(), local);
        }

        // 3. Full Sync (Fetch ALL calendars)
        // This ensures dependencies across calendars and background updates are caught.
        // RustyClient::get_all_tasks handles concurrency and delta sync (CTag/ETag), so this is efficient.
        match client.get_all_tasks(&cals).await {
            Ok(results) => {
                for (href, tasks) in results {
                    store.insert(href, tasks);
                }
            }
            Err(e) => {
                // If fetch fails (e.g. partial offline), try to load what we can from cache
                // and preserve the warning.
                for cal in &cals {
                    if cal.href != LOCAL_CALENDAR_HREF && !store.calendars.contains_key(&cal.href) {
                        if let Ok((cached, _)) = crate::cache::Cache::load(&cal.href) {
                            store.insert(cal.href.clone(), cached);
                        }
                    }
                }
                if warning.is_none() {
                    return Err(MobileError::from(e));
                }
            }
        }

        Ok(warning.unwrap_or_else(|| "Connected".to_string()))
    }

    async fn modify_task_and_sync<F>(&self, uid: String, mut modifier: F) -> Result<(), MobileError>
    where
        F: FnMut(&mut Task),
    {
        let mut store = self.store.lock().await;
        let (task, _) = store
            .get_task_mut(&uid)
            .ok_or(MobileError::from("Task not found"))?;
        modifier(task);
        let task_copy = task.clone();
        drop(store);
        let client_guard = self.client.lock().await;
        if let Some(client) = &*client_guard {
            client
                .update_task(&mut task_copy.clone())
                .await
                .map_err(MobileError::from)?;
        } else if task_copy.calendar_href == LOCAL_CALENDAR_HREF {
            let mut local = LocalStorage::load().unwrap_or_default();
            if let Some(idx) = local.iter().position(|t| t.uid == uid) {
                local[idx] = task_copy;
                LocalStorage::save(&local).map_err(MobileError::from)?;
            }
        }
        Ok(())
    }
}
