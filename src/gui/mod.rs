pub mod message;
pub mod state;
pub mod view;

use crate::cache::Cache;
use crate::client::RustyClient;
use crate::config::Config;
use crate::model::{CalendarListEntry, Task as TodoTask};
use crate::store::FilterOptions;

use chrono::{Duration, Utc};
use iced::{Element, Task, Theme, window};
use message::Message;
use state::{AppState, GuiApp, SidebarMode};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn run() -> iced::Result {
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    TOKIO_RUNTIME
        .set(runtime)
        .expect("Failed to set global runtime");

    iced::application(
        "Cfait | 🗹 Take control of your TODO list",
        GuiApp::update,
        GuiApp::view,
    )
    .theme(GuiApp::theme)
    .window(window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            #[cfg(target_os = "linux")]
            application_id: String::from("cfait"),

            ..Default::default()
        },
        ..Default::default()
    })
    .run_with(GuiApp::new)
}

impl GuiApp {
    fn new() -> (Self, Task<Message>) {
        (
            Self::default(),
            Task::perform(
                async { Config::load().map_err(|e| e.to_string()) },
                Message::ConfigLoaded,
            ),
        )
    }

    fn view(&self) -> Element<'_, Message> {
        view::root_view(self)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn save_config(&self) {
        let _ = Config {
            url: self.ob_url.clone(),
            username: self.ob_user.clone(),
            password: self.ob_pass.clone(),
            default_calendar: self.ob_default_cal.clone(),
            hide_completed: self.hide_completed,
            hide_fully_completed_tags: self.hide_fully_completed_tags,
            allow_insecure_certs: self.ob_insecure,
            hidden_calendars: self.hidden_calendars.iter().cloned().collect(),
            tag_aliases: self.tag_aliases.clone(),
            sort_cutoff_months: self.sort_cutoff_months,
        }
        .save();
    }

    // Helper to re-run filters
    fn refresh_filtered_tasks(&mut self) {
        let cal_filter = if self.sidebar_mode == SidebarMode::Categories {
            None
        } else {
            self.active_cal_href.as_deref()
        };

        let cutoff_date = if let Some(months) = self.sort_cutoff_months {
            // Basic calculation: Current UTC + Months * 30 Days
            let now = Utc::now();
            let days = months as i64 * 30;
            Some(now + Duration::days(days))
        } else {
            None
        };

        self.tasks = self.store.filter(FilterOptions {
            active_cal_href: cal_filter,
            hidden_calendars: &self.hidden_calendars,
            selected_categories: &self.selected_categories,
            match_all_categories: self.match_all_categories,
            search_term: &self.search_value,
            hide_completed_global: self.hide_completed,
            cutoff_date,
            min_duration: self.filter_min_duration,
            max_duration: self.filter_max_duration,
            include_unset_duration: self.filter_include_unset_duration,
        });
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigLoaded(Ok(config)) => {
                self.hidden_calendars = config.hidden_calendars.clone().into_iter().collect();
                self.sort_cutoff_months = config.sort_cutoff_months;
                self.ob_sort_months_input = match config.sort_cutoff_months {
                    Some(m) => m.to_string(),
                    None => "".to_string(),
                };
                self.ob_insecure = config.allow_insecure_certs;

                self.state = AppState::Loading;
                Task::perform(connect_and_fetch_wrapper(config), Message::Loaded)
            }
            Message::ConfigLoaded(Err(_)) => {
                self.state = AppState::Onboarding;
                Task::none()
            }

            Message::ObUrlChanged(v) => {
                self.ob_url = v;
                Task::none()
            }
            Message::ObUserChanged(v) => {
                self.ob_user = v;
                Task::none()
            }
            Message::ObPassChanged(v) => {
                self.ob_pass = v;
                Task::none()
            }
            Message::ObDefaultCalChanged(v) => {
                self.ob_default_cal = Some(v);
                Task::none()
            }
            Message::ObInsecureToggled(val) => {
                self.ob_insecure = val;
                Task::none()
            }

            Message::ObSubmit => {
                if self.ob_sort_months_input.trim().is_empty() {
                    self.sort_cutoff_months = None;
                } else if let Ok(n) = self.ob_sort_months_input.trim().parse::<u32>() {
                    self.sort_cutoff_months = Some(n);
                }

                // 1. Try to load the existing config to preserve settings like aliases.
                // If it fails (e.g., first run), create a new, empty config struct.
                let mut config_to_save = Config::load().unwrap_or_else(|_| Config {
                    url: String::new(),
                    username: String::new(),
                    password: String::new(),
                    default_calendar: None,
                    allow_insecure_certs: false,
                    hidden_calendars: Vec::new(),
                    hide_completed: self.hide_completed,
                    hide_fully_completed_tags: self.hide_fully_completed_tags,
                    tag_aliases: self.tag_aliases.clone(), // Use in-memory aliases if any
                    sort_cutoff_months: Some(6),
                });

                // 2. Modify the loaded (or new) config with the values from the UI.
                // This updates credentials without touching other saved settings.
                config_to_save.url = self.ob_url.clone();
                config_to_save.username = self.ob_user.clone();
                config_to_save.password = self.ob_pass.clone();
                config_to_save.default_calendar = self.ob_default_cal.clone();
                config_to_save.allow_insecure_certs = self.ob_insecure;
                config_to_save.hidden_calendars = self.hidden_calendars.iter().cloned().collect();
                config_to_save.hide_completed = self.hide_completed;
                config_to_save.hide_fully_completed_tags = self.hide_fully_completed_tags;
                config_to_save.tag_aliases = self.tag_aliases.clone();
                config_to_save.sort_cutoff_months = self.sort_cutoff_months;

                // 3. Save the merged configuration.
                let _ = config_to_save.save();

                self.state = AppState::Loading;
                self.error_msg = Some("Connecting...".to_string());

                // Use the newly saved config for the connection attempt.
                Task::perform(connect_and_fetch_wrapper(config_to_save), Message::Loaded)
            }

            // LOAD CONFIG ON OPEN SETTINGS
            Message::OpenSettings => {
                if let Ok(cfg) = Config::load() {
                    self.ob_url = cfg.url;
                    self.ob_user = cfg.username;
                    self.ob_pass = cfg.password;
                    self.ob_default_cal = cfg.default_calendar;
                    self.hide_completed = cfg.hide_completed;
                    self.hide_fully_completed_tags = cfg.hide_fully_completed_tags;
                    self.ob_insecure = cfg.allow_insecure_certs;
                    self.hidden_calendars = cfg.hidden_calendars.into_iter().collect();
                    self.tag_aliases = cfg.tag_aliases; // Load Map
                    self.sort_cutoff_months = cfg.sort_cutoff_months;
                    self.ob_sort_months_input = match cfg.sort_cutoff_months {
                        Some(m) => m.to_string(),
                        None => "".to_string(),
                    };
                }
                self.state = AppState::Settings;
                Task::none()
            }
            Message::CancelSettings => {
                self.state = AppState::Active;
                Task::none()
            }

            Message::Loaded(Ok((client, cals, tasks, active))) => {
                self.client = Some(client.clone());
                self.calendars = cals.clone();

                // Load config defaults if we haven't already
                if let Ok(cfg) = Config::load() {
                    self.hide_completed = cfg.hide_completed;
                    self.hide_fully_completed_tags = cfg.hide_fully_completed_tags;
                    self.tag_aliases = cfg.tag_aliases; // Load Map
                }

                self.store.clear();
                if let Some(href) = &active {
                    self.store.insert(href.clone(), tasks.clone());
                    let _ = Cache::save(href, &tasks);
                }

                self.active_cal_href = active.clone();

                // Auto-save successful connection params
                if !self.ob_url.is_empty() {
                    let _ = Config {
                        url: self.ob_url.clone(),
                        username: self.ob_user.clone(),
                        password: self.ob_pass.clone(),
                        default_calendar: self.ob_default_cal.clone(),
                        hide_completed: self.hide_completed,
                        hide_fully_completed_tags: self.hide_fully_completed_tags,
                        hidden_calendars: self.hidden_calendars.iter().cloned().collect(),
                        allow_insecure_certs: self.ob_insecure,
                        tag_aliases: self.tag_aliases.clone(),
                        sort_cutoff_months: self.sort_cutoff_months,
                    }
                    .save();
                }

                self.state = AppState::Active;
                self.error_msg = None;
                self.refresh_filtered_tasks();
                self.loading = true;

                Task::perform(async_fetch_all_wrapper(client, cals), Message::RefreshedAll)
            }
            Message::Loaded(Err(e)) => {
                self.error_msg = Some(format!("Connection Failed: {}", e));
                self.state = AppState::Onboarding;
                self.loading = false;
                Task::none()
            }

            Message::RefreshedAll(Ok(results)) => {
                for (href, tasks) in results {
                    self.store.insert(href.clone(), tasks.clone());
                    let _ = Cache::save(&href, &tasks);
                }
                self.refresh_filtered_tasks();
                self.loading = false;
                Task::none()
            }
            Message::RefreshedAll(Err(e)) => {
                self.error_msg = Some(format!("Sync warning: {}", e));
                self.loading = false;
                Task::none()
            }
            Message::TasksRefreshed(Ok(tasks)) => {
                if let Some(href) = &self.active_cal_href {
                    self.store.insert(href.clone(), tasks.clone());
                    let _ = Cache::save(href, &tasks);
                }
                self.refresh_filtered_tasks();
                self.loading = false;
                Task::none()
            }
            Message::TasksRefreshed(Err(e)) => {
                self.error_msg = Some(format!("Fetch: {}", e));
                self.loading = false;
                Task::none()
            }

            Message::SidebarModeChanged(mode) => {
                self.sidebar_mode = mode;
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::CategoryToggled(cat) => {
                if self.selected_categories.contains(&cat) {
                    self.selected_categories.remove(&cat);
                } else {
                    self.selected_categories.insert(cat);
                }
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::CategoryMatchModeChanged(val) => {
                self.match_all_categories = val;
                self.refresh_filtered_tasks();
                Task::none()
            }

            Message::ToggleHideCompleted(val) => {
                self.hide_completed = val;
                self.save_config(); // <--- PERSIST TO DISK
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::ToggleHideFullyCompletedTags(val) => {
                self.hide_fully_completed_tags = val;
                self.save_config(); // <--- PERSIST TO DISK
                self.refresh_filtered_tasks();
                Task::none()
            }

            Message::SelectCalendar(href) => {
                if self.sidebar_mode == SidebarMode::Categories {
                    self.sidebar_mode = SidebarMode::Calendars;
                }
                self.active_cal_href = Some(href.clone());
                self.refresh_filtered_tasks();

                if let Some(client) = &self.client {
                    self.loading = true;
                    return Task::perform(
                        async_fetch_wrapper(client.clone(), href),
                        Message::TasksRefreshed,
                    );
                }
                Task::none()
            }
            Message::SearchChanged(val) => {
                self.search_value = val;
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::InputChanged(value) => {
                self.input_value = value;
                Task::none()
            }
            Message::DescriptionChanged(value) => {
                self.description_value = value;
                Task::none()
            }

            Message::SubmitTask => {
                if !self.input_value.is_empty() {
                    if let Some(edit_uid) = &self.editing_uid {
                        let mut target_cal = None;
                        let mut target_idx = 0;
                        'outer: for (cal_href, tasks) in &self.store.calendars {
                            for (i, t) in tasks.iter().enumerate() {
                                if t.uid == *edit_uid {
                                    target_cal = Some(cal_href.clone());
                                    target_idx = i;
                                    break 'outer;
                                }
                            }
                        }

                        if let Some(cal_href) = target_cal
                            && let Some(tasks) = self.store.calendars.get_mut(&cal_href)
                        {
                            let task = &mut tasks[target_idx];
                            task.apply_smart_input(&self.input_value, &self.tag_aliases);
                            task.description = self.description_value.clone();

                            let task_copy = task.clone();
                            self.input_value.clear();
                            self.description_value.clear();
                            self.editing_uid = None;
                            self.refresh_filtered_tasks();

                            if let Some(client) = &self.client {
                                return Task::perform(
                                    async_update_wrapper(client.clone(), task_copy),
                                    Message::SyncSaved,
                                );
                            }
                        }
                    } else {
                        let mut new_task = TodoTask::new(&self.input_value, &self.tag_aliases);
                        let target_href = if let Some(h) = &self.active_cal_href {
                            h.clone()
                        } else if let Some(first) = self.calendars.first() {
                            first.href.clone()
                        } else {
                            String::new()
                        };

                        if !target_href.is_empty() {
                            new_task.calendar_href = target_href.clone();
                            self.store
                                .calendars
                                .entry(target_href)
                                .or_default()
                                .push(new_task.clone());
                            self.refresh_filtered_tasks();
                            self.input_value.clear();

                            if let Some(client) = &self.client {
                                return Task::perform(
                                    async_create_wrapper(client.clone(), new_task),
                                    Message::SyncSaved,
                                );
                            }
                        } else {
                            self.error_msg =
                                Some("No calendar available to create task".to_string());
                        }
                    }
                }
                Task::none()
            }
            Message::ToggleTask(index, _) => {
                if let Some(view_task) = self.tasks.get(index) {
                    let uid = view_task.uid.clone();
                    let cal_href = view_task.calendar_href.clone();

                    if let Some(cal_tasks) = self.store.calendars.get_mut(&cal_href)
                        && let Some(t) = cal_tasks.iter_mut().find(|t| t.uid == uid)
                    {
                        // Optimistic Toggle: Done <-> NeedsAction
                        let old_status = t.status;
                        t.status = if t.status == crate::model::TaskStatus::Completed {
                            crate::model::TaskStatus::NeedsAction
                        } else {
                            crate::model::TaskStatus::Completed
                        };

                        let mut server_task = t.clone();
                        server_task.status = old_status; // Revert for API call (API does the toggle)

                        self.refresh_filtered_tasks();

                        if let Some(client) = &self.client {
                            return Task::perform(
                                async_toggle_wrapper(client.clone(), server_task),
                                |res| Message::SyncToggleComplete(Box::new(res)),
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::AliasKeyInput(v) => {
                self.alias_input_key = v;
                Task::none()
            }
            Message::AliasValueInput(v) => {
                self.alias_input_values = v;
                Task::none()
            }
            Message::AddAlias => {
                if !self.alias_input_key.is_empty() && !self.alias_input_values.is_empty() {
                    // Parse comma separated values
                    let tags: Vec<String> = self
                        .alias_input_values
                        .split(',')
                        .map(|s| s.trim().trim_start_matches('#').to_string()) // Remove # if user typed it
                        .filter(|s| !s.is_empty())
                        .collect();

                    if !tags.is_empty() {
                        let key = self
                            .alias_input_key
                            .trim()
                            .trim_start_matches('#')
                            .to_string();
                        self.tag_aliases.insert(key, tags);
                        self.alias_input_key.clear();
                        self.alias_input_values.clear();
                        self.save_config();
                    }
                }
                Task::none()
            }
            Message::RemoveAlias(key) => {
                self.tag_aliases.remove(&key);
                self.save_config();
                Task::none()
            }

            Message::ObSortMonthsChanged(val) => {
                // Only allow numbers
                if val.is_empty() || val.chars().all(|c| c.is_numeric()) {
                    self.ob_sort_months_input = val;
                }
                Task::none()
            }

            Message::SyncSaved(Ok(updated)) => {
                if let Some(tasks) = self.store.calendars.get_mut(&updated.calendar_href)
                    && let Some(idx) = tasks.iter().position(|t| t.uid == updated.uid)
                {
                    tasks[idx] = updated.clone();
                    let _ = Cache::save(&updated.calendar_href, tasks);
                }
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::SyncSaved(Err(e)) => {
                self.error_msg = Some(format!("Sync Error: {}", e));
                Task::none()
            }
            Message::SyncToggleComplete(boxed_res) => match *boxed_res {
                Ok((updated, created_opt)) => {
                    if let Some(tasks) = self.store.calendars.get_mut(&updated.calendar_href) {
                        if let Some(idx) = tasks.iter().position(|t| t.uid == updated.uid) {
                            tasks[idx] = updated.clone();
                        }
                        if let Some(created) = created_opt {
                            tasks.push(created);
                        }
                        let _ = Cache::save(&updated.calendar_href, tasks);
                    }
                    self.refresh_filtered_tasks();
                    Task::none()
                }
                Err(e) => {
                    self.error_msg = Some(format!("Toggle Error: {}", e));
                    Task::none()
                }
            },
            Message::EditTaskStart(index) => {
                if let Some(task) = self.tasks.get(index) {
                    self.input_value = task.to_smart_string();
                    self.description_value = task.description.clone();
                    self.editing_uid = Some(task.uid.clone());
                }
                Task::none()
            }
            Message::CancelEdit => {
                self.input_value.clear();
                self.description_value.clear();
                self.editing_uid = None;
                Task::none()
            }
            Message::DeleteTask(index) => {
                if let Some(task) = self.tasks.get(index).cloned() {
                    if let Some(tasks) = self.store.calendars.get_mut(&task.calendar_href) {
                        tasks.retain(|t| t.uid != task.uid);
                        let _ = Cache::save(&task.calendar_href, tasks);
                    }
                    self.refresh_filtered_tasks();
                    if let Some(client) = &self.client {
                        return Task::perform(
                            async_delete_wrapper(client.clone(), task),
                            Message::DeleteComplete,
                        );
                    }
                }
                Task::none()
            }
            Message::DeleteComplete(_) => Task::none(),
            Message::ChangePriority(index, delta) => {
                if let Some(view_task) = self.tasks.get(index) {
                    let uid = view_task.uid.clone();
                    let cal_href = view_task.calendar_href.clone();

                    if let Some(tasks) = self.store.calendars.get_mut(&cal_href)
                        && let Some(t) = tasks.iter_mut().find(|t| t.uid == uid)
                    {
                        let new_prio = if delta > 0 {
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
                        t.priority = new_prio;
                        let t_clone = t.clone();
                        self.refresh_filtered_tasks();
                        if let Some(client) = &self.client {
                            return Task::perform(
                                async_update_wrapper(client.clone(), t_clone),
                                Message::SyncSaved,
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::SetTaskStatus(index, new_status) => {
                if let Some(view_task) = self.tasks.get(index) {
                    let uid = view_task.uid.clone();
                    let cal_href = view_task.calendar_href.clone();

                    if let Some(cal_tasks) = self.store.calendars.get_mut(&cal_href)
                        && let Some(t) = cal_tasks.iter_mut().find(|t| t.uid == uid)
                    {
                        t.status = new_status;
                        let t_clone = t.clone();
                        self.refresh_filtered_tasks();
                        if let Some(client) = &self.client {
                            return Task::perform(
                                async_update_wrapper(client.clone(), t_clone),
                                Message::SyncSaved,
                            );
                        }
                    }
                }
                Task::none()
            }

            Message::SetMinDuration(val) => {
                self.filter_min_duration = val;
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::SetMaxDuration(val) => {
                self.filter_max_duration = val;
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::ToggleIncludeUnsetDuration(val) => {
                self.filter_include_unset_duration = val;
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::ToggleDetails(uid) => {
                if self.expanded_tasks.contains(&uid) {
                    self.expanded_tasks.remove(&uid);
                } else {
                    self.expanded_tasks.insert(uid);
                }
                Task::none()
            }
            Message::YankTask(uid) => {
                self.yanked_uid = Some(uid);
                Task::none()
            }

            Message::ClearYank => {
                self.yanked_uid = None;
                Task::none()
            }

            Message::MakeChild(target_uid) => {
                if let Some(parent_uid) = &self.yanked_uid {
                    // Standard find-and-update logic
                    let mut target_cal = None;
                    let mut target_idx = 0;

                    'outer: for (cal_href, tasks) in &self.store.calendars {
                        for (i, t) in tasks.iter().enumerate() {
                            if t.uid == target_uid {
                                target_cal = Some(cal_href.clone());
                                target_idx = i;
                                break 'outer;
                            }
                        }
                    }

                    if let Some(cal_href) = target_cal
                        && let Some(tasks) = self.store.calendars.get_mut(&cal_href)
                    {
                        let task = &mut tasks[target_idx];

                        // Prevent self-parenting or redundancy
                        if task.uid != *parent_uid && task.parent_uid.as_ref() != Some(parent_uid) {
                            task.parent_uid = Some(parent_uid.clone());

                            let task_copy = task.clone();
                            self.refresh_filtered_tasks(); // Refresh UI tree immediately

                            if let Some(client) = &self.client {
                                return Task::perform(
                                    async_update_wrapper(client.clone(), task_copy),
                                    Message::SyncSaved,
                                );
                            }
                        }
                    }
                }
                Task::none()
            }

            Message::RemoveParent(child_uid) => {
                let mut target_cal = None;
                let mut target_idx = 0;

                // Find the task
                'outer_p: for (cal_href, tasks) in &self.store.calendars {
                    for (i, t) in tasks.iter().enumerate() {
                        if t.uid == child_uid {
                            target_cal = Some(cal_href.clone());
                            target_idx = i;
                            break 'outer_p;
                        }
                    }
                }

                if let Some(cal_href) = target_cal
                    && let Some(tasks) = self.store.calendars.get_mut(&cal_href)
                {
                    let task = &mut tasks[target_idx];
                    task.parent_uid = None; // Detach

                    let task_copy = task.clone();
                    self.refresh_filtered_tasks();

                    if let Some(client) = &self.client {
                        return Task::perform(
                            async_update_wrapper(client.clone(), task_copy),
                            Message::SyncSaved,
                        );
                    }
                }
                Task::none()
            }

            Message::RemoveDependency(task_uid, dep_uid) => {
                let mut target_cal = None;
                let mut target_idx = 0;

                'outer_d: for (cal_href, tasks) in &self.store.calendars {
                    for (i, t) in tasks.iter().enumerate() {
                        if t.uid == task_uid {
                            target_cal = Some(cal_href.clone());
                            target_idx = i;
                            break 'outer_d;
                        }
                    }
                }

                if let Some(cal_href) = target_cal
                    && let Some(tasks) = self.store.calendars.get_mut(&cal_href)
                {
                    let task = &mut tasks[target_idx];
                    task.dependencies.retain(|d| *d != dep_uid); // Remove dependency

                    let task_copy = task.clone();
                    self.refresh_filtered_tasks();

                    if let Some(client) = &self.client {
                        return Task::perform(
                            async_update_wrapper(client.clone(), task_copy),
                            Message::SyncSaved,
                        );
                    }
                }
                Task::none()
            }

            Message::AddDependency(target_uid) => {
                if let Some(blocker_uid) = &self.yanked_uid {
                    // 1. Find target task in store
                    // We need to scan all calendars because we don't know which cal the target is in
                    // (unless we pass it, but scanning store is fast enough for GUI)
                    let mut target_cal = None;
                    let mut target_idx = 0;

                    'outer: for (cal_href, tasks) in &self.store.calendars {
                        for (i, t) in tasks.iter().enumerate() {
                            if t.uid == target_uid {
                                target_cal = Some(cal_href.clone());
                                target_idx = i;
                                break 'outer;
                            }
                        }
                    }

                    if let Some(cal_href) = target_cal
                        && let Some(tasks) = self.store.calendars.get_mut(&cal_href)
                    {
                        let task = &mut tasks[target_idx];

                        // 2. Check if already exists or self-ref
                        if task.uid != *blocker_uid && !task.dependencies.contains(blocker_uid) {
                            task.dependencies.push(blocker_uid.clone());

                            // 3. Save & Refresh
                            let task_copy = task.clone();
                            self.refresh_filtered_tasks();

                            if let Some(client) = &self.client {
                                return Task::perform(
                                    async_update_wrapper(client.clone(), task_copy),
                                    Message::SyncSaved,
                                );
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ToggleCalendarVisibility(href, is_visible) => {
                if is_visible {
                    self.hidden_calendars.remove(&href);
                } else {
                    self.hidden_calendars.insert(href);
                }
                // Save immediately so it persists
                self.save_config();
                // Refresh main view if we just hid the active calendar
                if let Some(active) = &self.active_cal_href {
                    if self.hidden_calendars.contains(active) {
                        self.active_cal_href = None;
                    }
                }
                self.refresh_filtered_tasks();
                Task::none()
            }
            Message::MoveTask(task_uid, target_href) => {
                // 1. Find the task
                let mut task_to_move = None;
                // We need to find which calendar it currently belongs to
                'search: for tasks in self.store.calendars.values() {
                    if let Some(t) = tasks.iter().find(|t| t.uid == task_uid) {
                        task_to_move = Some(t.clone());
                        break 'search;
                    }
                }

                if let Some(task) = task_to_move {
                    if task.calendar_href == target_href {
                        return Task::none(); // Moving to same calendar, do nothing
                    }

                    // 2. Optimistic UI Update
                    // Remove from old list
                    if let Some(old_list) = self.store.calendars.get_mut(&task.calendar_href) {
                        old_list.retain(|t| t.uid != task_uid);
                        let _ = Cache::save(&task.calendar_href, old_list);
                    }

                    // Add to new list (locally constructed version)
                    let mut local_moved = task.clone();
                    local_moved.calendar_href = target_href.clone();
                    self.store
                        .calendars
                        .entry(target_href.clone())
                        .or_default()
                        .push(local_moved);
                    // Note: We don't save cache for new list yet, waiting for server ETag

                    self.refresh_filtered_tasks();

                    // 3. API Call
                    if let Some(client) = &self.client {
                        return Task::perform(
                            async_move_wrapper(client.clone(), task, target_href),
                            Message::TaskMoved,
                        );
                    }
                }
                Task::none()
            }

            Message::TaskMoved(Ok(new_task)) => {
                // Server confirmed creation. Update the store with the "real" task (w/ ETag)
                if let Some(list) = self.store.calendars.get_mut(&new_task.calendar_href) {
                    // Replace the optimistic one (which has no ETag) with the real one
                    if let Some(idx) = list.iter().position(|t| t.uid == new_task.uid) {
                        list[idx] = new_task.clone();
                    } else {
                        // Should be there from optimistic update, but just in case
                        list.push(new_task.clone());
                    }
                    let _ = Cache::save(&new_task.calendar_href, list);
                }
                self.refresh_filtered_tasks();
                Task::none()
            }

            Message::TaskMoved(Err(e)) => {
                self.error_msg = Some(format!("Move failed: {}", e));
                // Ideally: Revert optimistic update here (reload from disk/network)
                Task::none()
            }
        }
    }
}

// --- WRAPPERS ---

async fn connect_and_fetch_wrapper(
    config: Config,
) -> Result<
    (
        RustyClient,
        Vec<CalendarListEntry>,
        Vec<TodoTask>,
        Option<String>,
    ),
    String,
> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async { connect_and_fetch(config).await })
        .await
        .map_err(|e| e.to_string())?
}

async fn async_fetch_wrapper(client: RustyClient, href: String) -> Result<Vec<TodoTask>, String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move {
        let tasks = client.get_tasks(&href).await.map_err(|e| e.to_string())?;
        Ok(tasks)
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn async_fetch_all_wrapper(
    client: RustyClient,
    cals: Vec<CalendarListEntry>,
) -> Result<Vec<(String, Vec<TodoTask>)>, String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move { client.get_all_tasks(&cals).await })
        .await
        .map_err(|e| e.to_string())?
}

async fn async_create_wrapper(client: RustyClient, task: TodoTask) -> Result<TodoTask, String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move { async_create(client, task).await })
        .await
        .map_err(|e| e.to_string())?
}
async fn async_update_wrapper(client: RustyClient, task: TodoTask) -> Result<TodoTask, String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move { async_update(client, task).await })
        .await
        .map_err(|e| e.to_string())?
}
async fn async_delete_wrapper(client: RustyClient, task: TodoTask) -> Result<(), String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move { client.delete_task(&task).await })
        .await
        .map_err(|e| e.to_string())?
}
async fn async_toggle_wrapper(
    client: RustyClient,
    mut task: TodoTask,
) -> Result<(TodoTask, Option<TodoTask>), String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move { client.toggle_task(&mut task).await })
        .await
        .map_err(|e| e.to_string())?
}

async fn connect_and_fetch(
    config: Config,
) -> Result<
    (
        RustyClient,
        Vec<CalendarListEntry>,
        Vec<TodoTask>,
        Option<String>,
    ),
    String,
> {
    // This call is simple and should not have the complex error mapping.
    let client = RustyClient::new(
        &config.url,
        &config.username,
        &config.password,
        config.allow_insecure_certs,
    )
    .map_err(|e| e.to_string())?;

    let calendars = client.get_calendars().await.map_err(|e| {
        let err_str = e.to_string();
        // Check for common self-signed certificate errors.
        if err_str.contains("InvalidCertificate") {
            let mut helpful_msg =
                "Connection failed: The server presented an invalid TLS/SSL certificate.".to_string();

            // Give different advice based on whether the user has already tried the insecure option.
            if !config.allow_insecure_certs {
                helpful_msg.push_str(
                    "\n\nIf you are connecting to a self-hosted server (like Radicale), try enabling the 'Allow Insecure SSL' option in Settings.",
                );
            } else {
                helpful_msg.push_str(
                    "\n\nEven with 'Allow Insecure SSL' enabled, the certificate is structurally invalid (e.g., a CA certificate is being used as a server certificate). Please check your server's TLS/SSL configuration.",
                );
            }
            // Always include the technical details for power-users.
            helpful_msg.push_str(&format!("\n\nDetails: {}", err_str));
            helpful_msg
        } else {
            // If it's not a certificate error, just pass it through.
            err_str
        }
    })?;

    // The rest of the function proceeds as before.
    let mut active_href = None;
    if let Some(def_cal) = &config.default_calendar {
        if let Some(found) = calendars
            .iter()
            .find(|c| c.name == *def_cal || c.href == *def_cal)
        {
            active_href = Some(found.href.clone());
        } else if let Ok(href) = client.discover_calendar().await {
            active_href = Some(href);
        }
    } else if let Ok(href) = client.discover_calendar().await {
        active_href = Some(href);
    }

    let tasks = if let Some(ref h) = active_href {
        client.get_tasks(h).await.map_err(|e| e.to_string())?
    } else {
        vec![]
    };
    Ok((client, calendars, tasks, active_href))
}
async fn async_create(client: RustyClient, mut task: TodoTask) -> Result<TodoTask, String> {
    client.create_task(&mut task).await?;
    Ok(task)
}
async fn async_update(client: RustyClient, mut task: TodoTask) -> Result<TodoTask, String> {
    client.update_task(&mut task).await?;
    Ok(task)
}

async fn async_move_wrapper(
    client: RustyClient,
    task: TodoTask,
    new_href: String,
) -> Result<TodoTask, String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move { client.move_task(&task, &new_href).await })
        .await
        .map_err(|e| e.to_string())?
}
