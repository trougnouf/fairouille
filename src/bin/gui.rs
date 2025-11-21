use rustache::client::RustyClient;
use rustache::config::Config;
use rustache::model::{CalendarListEntry, Task as TodoTask};

use iced::widget::{Rule, button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Background, Color, Element, Length, Task, Theme};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn main() -> iced::Result {
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    TOKIO_RUNTIME
        .set(runtime)
        .expect("Failed to set global runtime");

    iced::application("Rustache", RustacheGui::update, RustacheGui::view)
        .theme(RustacheGui::theme)
        .run_with(RustacheGui::new)
}

struct RustacheGui {
    tasks: Vec<TodoTask>,
    calendars: Vec<CalendarListEntry>, // <--- Store Calendars
    active_cal_href: Option<String>,   // <--- Track Active

    input_value: String,
    client: Option<RustyClient>,
    loading: bool,
    error_msg: Option<String>,
}

impl Default for RustacheGui {
    fn default() -> Self {
        Self {
            tasks: vec![],
            calendars: vec![],
            active_cal_href: None,
            input_value: String::new(),
            client: None,
            loading: true,
            error_msg: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    CreateTask,
    ToggleTask(usize, bool),
    SelectCalendar(String), // <--- New Action

    // Async Events
    // Returns: Client, Calendars, Tasks, Active Href
    Loaded(
        Result<
            (
                RustyClient,
                Vec<CalendarListEntry>,
                Vec<TodoTask>,
                Option<String>,
            ),
            String,
        >,
    ),
    SyncSaved(Result<TodoTask, String>),
    TasksRefreshed(Result<Vec<TodoTask>, String>),
}

impl RustacheGui {
    fn new() -> (Self, Task<Message>) {
        (
            Self::default(),
            Task::perform(connect_and_fetch_wrapper(), Message::Loaded),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // --- ASYNC HANDLERS ---
            Message::Loaded(Ok((client, cals, tasks, active))) => {
                self.client = Some(client);
                self.calendars = cals;
                self.tasks = tasks;
                self.active_cal_href = active;
                self.loading = false;
            }
            Message::Loaded(Err(e)) => {
                self.error_msg = Some(format!("Connection Failed: {}", e));
                self.loading = false;
            }

            Message::SyncSaved(Ok(updated_task)) => {
                if let Some(index) = self.tasks.iter().position(|t| t.uid == updated_task.uid) {
                    self.tasks[index] = updated_task;
                }
            }
            Message::SyncSaved(Err(e)) => {
                self.error_msg = Some(format!("Sync Error: {}", e));
            }

            Message::TasksRefreshed(Ok(tasks)) => {
                self.tasks = tasks;
                self.loading = false;
            }
            Message::TasksRefreshed(Err(e)) => {
                self.error_msg = Some(format!("Fetch Error: {}", e));
                self.loading = false;
            }

            // --- UI HANDLERS ---

            // NEW: Switch Calendar
            Message::SelectCalendar(href) => {
                if let Some(client) = &mut self.client {
                    self.loading = true;
                    self.active_cal_href = Some(href.clone());

                    // Update client state immediately so creates go to right place
                    client.set_calendar(&href);

                    return Task::perform(
                        async_fetch_wrapper(client.clone()),
                        Message::TasksRefreshed,
                    );
                }
            }

            Message::InputChanged(value) => {
                self.input_value = value;
            }

            Message::CreateTask => {
                if !self.input_value.is_empty() {
                    let new_task = TodoTask::new(&self.input_value);
                    self.tasks.push(new_task.clone());
                    self.input_value.clear();

                    if let Some(client) = &self.client {
                        return Task::perform(
                            async_create_wrapper(client.clone(), new_task),
                            Message::SyncSaved,
                        );
                    }
                }
            }

            Message::ToggleTask(index, is_checked) => {
                if let Some(task) = self.tasks.get_mut(index) {
                    task.completed = is_checked;

                    if let Some(client) = &self.client {
                        return Task::perform(
                            async_update_wrapper(client.clone(), task.clone()),
                            Message::SyncSaved,
                        );
                    }
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // 1. SIDEBAR
        let sidebar_content = column(
            self.calendars
                .iter()
                .map(|cal| {
                    let is_active = self.active_cal_href.as_ref() == Some(&cal.href);

                    // Style the button based on activity
                    let btn = button(text(&cal.name).size(16))
                        .padding(10)
                        .width(Length::Fill)
                        .on_press(Message::SelectCalendar(cal.href.clone()));

                    // Simple style tweak for active
                    if is_active {
                        btn.style(button::primary)
                    } else {
                        btn.style(button::secondary)
                    }
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(10)
        .padding(10);

        let sidebar = container(scrollable(sidebar_content))
            .width(200) // Fixed width sidebar
            .height(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style::default()
                    .background(Background::Color(palette.background.weak.color))
            });

        // 2. MAIN CONTENT
        let title_text = if self.loading {
            "Loading..."
        } else {
            "Rustache"
        };

        let input = text_input("Add a task (e.g. Buy Milk !1)...", &self.input_value)
            .on_input(Message::InputChanged)
            .on_submit(Message::CreateTask)
            .padding(10)
            .size(20);

        let tasks_view: Element<_> = column(
            self.tasks
                .iter()
                .enumerate()
                .map(|(i, task)| {
                    let color = match task.priority {
                        1..=4 => Color::from_rgb(0.8, 0.2, 0.2),
                        5 => Color::from_rgb(0.8, 0.8, 0.2),
                        _ => Color::WHITE,
                    };

                    row![
                        checkbox("", task.completed).on_toggle(move |b| Message::ToggleTask(i, b)),
                        text(&task.summary).size(20).color(color),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center)
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(10)
        .into();

        let main_content = column![text(title_text).size(40), input, scrollable(tasks_view)]
            .spacing(20)
            .padding(20)
            .max_width(800);

        // 3. ASSEMBLE LAYOUT
        let layout = row![
            sidebar,
            Rule::vertical(1),
            container(main_content)
                .width(Length::Fill)
                .center_x(Length::Fill)
        ];

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

// --- WRAPPERS ---

async fn connect_and_fetch_wrapper() -> Result<
    (
        RustyClient,
        Vec<CalendarListEntry>,
        Vec<TodoTask>,
        Option<String>,
    ),
    String,
> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async { connect_and_fetch().await })
        .await
        .map_err(|e| e.to_string())?
}

async fn async_fetch_wrapper(client: RustyClient) -> Result<Vec<TodoTask>, String> {
    let rt = TOKIO_RUNTIME.get().expect("Runtime not initialized");
    rt.spawn(async move {
        let mut tasks = client.get_tasks().await.map_err(|e| e.to_string())?;
        tasks.sort();
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

// --- LOGIC ---

async fn connect_and_fetch() -> Result<
    (
        RustyClient,
        Vec<CalendarListEntry>,
        Vec<TodoTask>,
        Option<String>,
    ),
    String,
> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let mut client = RustyClient::new(&config.url, &config.username, &config.password)
        .map_err(|e| e.to_string())?;

    // 1. Get Calendars
    let calendars = client.get_calendars().await.unwrap_or_default();
    let mut active_href = None;

    // 2. Select Default
    if let Some(def_cal) = config.default_calendar {
        if let Some(found) = calendars
            .iter()
            .find(|c| c.name == def_cal || c.href == def_cal)
        {
            client.set_calendar(&found.href);
            active_href = Some(found.href.clone());
        } else {
            // Fallback
            if let Ok(href) = client.discover_calendar().await {
                active_href = Some(href);
            }
        }
    } else {
        if let Ok(href) = client.discover_calendar().await {
            active_href = Some(href);
        }
    }

    // 3. Fetch Tasks
    let mut tasks = client.get_tasks().await.map_err(|e| e.to_string())?;
    tasks.sort();

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
