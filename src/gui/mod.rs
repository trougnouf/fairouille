pub mod message;
pub mod state;
pub mod view;

use crate::cache::Cache;
use crate::client::RustyClient;
use crate::config::Config;
use crate::model::{CalendarListEntry, Task as TodoTask};

use iced::{Element, Task, Theme, window};
use message::Message;
use state::{AppState, GuiApp};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn run() -> iced::Result {
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    TOKIO_RUNTIME
        .set(runtime)
        .expect("Failed to set global runtime");

    iced::application("Fairouille", GuiApp::update, GuiApp::view)
        .theme(GuiApp::theme)
        .window(window::Settings {
            platform_specific: window::settings::PlatformSpecific {
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

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigLoaded(Ok(config)) => {
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
            Message::OpenSettings => {
                if let Ok(cfg) = Config::load() {
                    self.ob_url = cfg.url;
                    self.ob_user = cfg.username;
                    self.ob_pass = cfg.password;
                    self.ob_default_cal = cfg.default_calendar;
                }
                self.state = AppState::Settings;
                Task::none()
            }
            Message::CancelSettings => {
                self.state = AppState::Active;
                Task::none()
            }
            Message::ObSubmit => {
                let config = Config {
                    url: self.ob_url.clone(),
                    username: self.ob_user.clone(),
                    password: self.ob_pass.clone(),
                    default_calendar: self.ob_default_cal.clone(),
                };
                self.state = AppState::Loading;
                self.error_msg = Some("Connecting...".to_string());
                Task::perform(connect_and_fetch_wrapper(config), Message::Loaded)
            }
            Message::Loaded(Ok((client, cals, tasks, active))) => {
                self.client = Some(client);
                self.calendars = cals;
                self.tasks = TodoTask::organize_hierarchy(tasks.clone());
                self.active_cal_href = active.clone();
                if let Some(href) = &active {
                    let _ = Cache::save(href, &tasks);
                }
                if !self.ob_url.is_empty() {
                    let _ = Config {
                        url: self.ob_url.clone(),
                        username: self.ob_user.clone(),
                        password: self.ob_pass.clone(),
                        default_calendar: self.ob_default_cal.clone(),
                    }
                    .save();
                }
                self.state = AppState::Active;
                self.error_msg = None;
                self.loading = false;
                Task::none()
            }
            Message::Loaded(Err(e)) => {
                self.error_msg = Some(format!("Connection Failed: {}", e));
                self.state = AppState::Onboarding;
                self.loading = false;
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
            Message::SyncSaved(Ok(updated_task)) => {
                if let Some(index) = self.tasks.iter().position(|t| t.uid == updated_task.uid) {
                    self.tasks[index] = updated_task;
                    let raw = self.tasks.clone();
                    self.tasks = TodoTask::organize_hierarchy(raw);
                    if let Some(href) = &self.active_cal_href {
                        let _ = Cache::save(href, &self.tasks);
                    }
                }
                Task::none()
            }
            Message::SyncSaved(Err(e)) => {
                self.error_msg = Some(format!("Sync Error: {}", e));
                Task::none()
            }
            Message::SyncToggleComplete(Ok((updated, created_opt))) => {
                if let Some(index) = self.tasks.iter().position(|t| t.uid == updated.uid) {
                    self.tasks[index] = updated;
                }
                if let Some(created) = created_opt {
                    self.tasks.push(created);
                }
                let raw = self.tasks.clone();
                self.tasks = TodoTask::organize_hierarchy(raw);
                if let Some(href) = &self.active_cal_href {
                    let _ = Cache::save(href, &self.tasks);
                }
                Task::none()
            }
            Message::SyncToggleComplete(Err(e)) => {
                self.error_msg = Some(format!("Toggle Error: {}", e));
                Task::none()
            }
            Message::TasksRefreshed(Ok(tasks)) => {
                self.tasks = TodoTask::organize_hierarchy(tasks.clone());
                if let Some(href) = &self.active_cal_href {
                    let _ = Cache::save(href, &tasks);
                }
                self.loading = false;
                Task::none()
            }
            Message::TasksRefreshed(Err(e)) => {
                self.error_msg = Some(format!("Fetch: {}", e));
                self.loading = false;
                Task::none()
            }
            Message::SelectCalendar(href) => {
                if let Some(client) = &mut self.client {
                    self.active_cal_href = Some(href.clone());
                    if let Ok(cached) = Cache::load(&href) {
                        self.tasks = TodoTask::organize_hierarchy(cached);
                    } else {
                        self.tasks.clear();
                    }
                    self.loading = true;
                    return Task::perform(
                        async_fetch_wrapper(client.clone(), href.clone()),
                        Message::TasksRefreshed,
                    );
                }
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
            Message::SearchChanged(val) => {
                self.search_value = val;
                Task::none()
            }
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
            Message::SubmitTask => {
                if !self.input_value.is_empty() {
                    if let Some(edit_uid) = &self.editing_uid {
                        if let Some(index) = self.tasks.iter().position(|t| t.uid == *edit_uid) {
                            if let Some(task) = self.tasks.get_mut(index) {
                                task.apply_smart_input(&self.input_value);
                                task.description = self.description_value.clone();
                                let task_copy = task.clone();
                                self.input_value.clear();
                                self.description_value.clear();
                                self.editing_uid = None;
                                if let Some(client) = &self.client {
                                    return Task::perform(
                                        async_update_wrapper(client.clone(), task_copy),
                                        Message::SyncSaved,
                                    );
                                }
                            }
                        }
                    } else {
                        let mut new_task = TodoTask::new(&self.input_value);
                        if let Some(cal_href) = &self.active_cal_href {
                            new_task.calendar_href = cal_href.clone();
                        }
                        self.tasks.push(new_task.clone());
                        let raw = self.tasks.clone();
                        self.tasks = TodoTask::organize_hierarchy(raw);
                        self.input_value.clear();
                        if let Some(client) = &self.client {
                            if new_task.calendar_href.is_empty() {
                                self.error_msg = Some("No calendar selected".to_string());
                            } else {
                                return Task::perform(
                                    async_create_wrapper(client.clone(), new_task),
                                    Message::SyncSaved,
                                );
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ToggleTask(index, _checked) => {
                if let Some(task) = self.tasks.get_mut(index) {
                    task.completed = !task.completed;
                    let mut task_for_server = task.clone();
                    task_for_server.completed = !task_for_server.completed;
                    if let Some(client) = &self.client {
                        return Task::perform(
                            async_toggle_wrapper(client.clone(), task_for_server),
                            Message::SyncToggleComplete,
                        );
                    }
                }
                Task::none()
            }
            Message::DeleteTask(index) => {
                if let Some(task) = self.tasks.get(index).cloned() {
                    self.tasks.remove(index);
                    let raw = self.tasks.clone();
                    self.tasks = TodoTask::organize_hierarchy(raw);
                    if let Some(href) = &self.active_cal_href {
                        let _ = Cache::save(href, &self.tasks);
                    }
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
                if let Some(task) = self.tasks.get_mut(index) {
                    let new_prio = if delta > 0 {
                        match task.priority {
                            0 => 9,
                            9 => 5,
                            5 => 1,
                            1 => 1,
                            _ => 5,
                        }
                    } else {
                        match task.priority {
                            1 => 5,
                            5 => 9,
                            9 => 0,
                            0 => 0,
                            _ => 0,
                        }
                    };
                    if new_prio != task.priority {
                        task.priority = new_prio;
                        if let Some(client) = &self.client {
                            return Task::perform(
                                async_update_wrapper(client.clone(), task.clone()),
                                Message::SyncSaved,
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::IndentTask(index) => {
                if index > 0 && index < self.tasks.len() {
                    let parent_uid = self.tasks[index - 1].uid.clone();
                    if self.tasks[index].parent_uid != Some(parent_uid.clone()) {
                        if let Some(task) = self.tasks.get_mut(index) {
                            task.parent_uid = Some(parent_uid);
                            let task_copy = task.clone();
                            let raw = self.tasks.clone();
                            self.tasks = TodoTask::organize_hierarchy(raw);
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
            Message::OutdentTask(index) => {
                if let Some(task) = self.tasks.get_mut(index) {
                    if task.parent_uid.is_some() {
                        task.parent_uid = None;
                        let task_copy = task.clone();
                        let raw = self.tasks.clone();
                        self.tasks = TodoTask::organize_hierarchy(raw);
                        if let Some(client) = &self.client {
                            return Task::perform(
                                async_update_wrapper(client.clone(), task_copy),
                                Message::SyncSaved,
                            );
                        }
                    }
                }
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
    let client = RustyClient::new(&config.url, &config.username, &config.password)
        .map_err(|e| e.to_string())?;
    let calendars = client.get_calendars().await.unwrap_or_default();
    let mut active_href = None;
    if let Some(def_cal) = &config.default_calendar {
        if let Some(found) = calendars
            .iter()
            .find(|c| c.name == *def_cal || c.href == *def_cal)
        {
            active_href = Some(found.href.clone());
        } else {
            if let Ok(href) = client.discover_calendar().await {
                active_href = Some(href);
            }
        }
    } else {
        if let Ok(href) = client.discover_calendar().await {
            active_href = Some(href);
        }
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
