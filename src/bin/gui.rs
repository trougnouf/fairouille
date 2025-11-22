use rustache::client::RustyClient;
use rustache::config::Config;
use rustache::model::{CalendarListEntry, Task as TodoTask};

use iced::widget::{
    Rule, button, checkbox, column, container, horizontal_space, row, scrollable, text, text_input,
};
use iced::{Background, Color, Element, Length, Padding, Task, Theme};
use std::collections::HashSet;
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
    calendars: Vec<CalendarListEntry>,
    active_cal_href: Option<String>,
    input_value: String,
    description_value: String,
    search_value: String,
    editing_uid: Option<String>,
    expanded_tasks: HashSet<String>,
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
            description_value: String::new(),
            search_value: String::new(),
            editing_uid: None,
            expanded_tasks: HashSet::new(),
            client: None,
            loading: true,
            error_msg: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    DescriptionChanged(String),
    SearchChanged(String),
    SubmitTask,
    ToggleTask(usize, bool),
    SelectCalendar(String),
    DeleteTask(usize),
    EditTaskStart(usize),
    CancelEdit,
    ChangePriority(usize, i8),
    IndentTask(usize),
    OutdentTask(usize),
    ToggleDetails(String),
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
    SyncToggleComplete(Result<(TodoTask, Option<TodoTask>), String>),
    TasksRefreshed(Result<Vec<TodoTask>, String>),
    DeleteComplete(#[allow(dead_code)] Result<(), String>),
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
            Message::ToggleDetails(uid) => {
                if self.expanded_tasks.contains(&uid) {
                    self.expanded_tasks.remove(&uid);
                } else {
                    self.expanded_tasks.insert(uid);
                }
                Task::none()
            }
            Message::Loaded(Ok((client, cals, tasks, active))) => {
                self.client = Some(client);
                self.calendars = cals;
                self.tasks = TodoTask::organize_hierarchy(tasks);
                self.active_cal_href = active;
                self.loading = false;
                Task::none()
            }
            Message::Loaded(Err(e)) => {
                self.error_msg = Some(format!("Connect: {}", e));
                self.loading = false;
                Task::none()
            }

            Message::SyncSaved(Ok(updated_task)) => {
                if let Some(index) = self.tasks.iter().position(|t| t.uid == updated_task.uid) {
                    self.tasks[index] = updated_task;
                    let raw = self.tasks.clone();
                    self.tasks = TodoTask::organize_hierarchy(raw);
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
                Task::none()
            }
            Message::SyncToggleComplete(Err(e)) => {
                self.error_msg = Some(format!("Toggle Error: {}", e));
                Task::none()
            }

            Message::TasksRefreshed(Ok(tasks)) => {
                self.tasks = TodoTask::organize_hierarchy(tasks);
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
                    self.loading = true;
                    self.active_cal_href = Some(href.clone());
                    client.set_calendar(&href);
                    return Task::perform(
                        async_fetch_wrapper(client.clone()),
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
                        let new_task = TodoTask::new(&self.input_value);
                        self.tasks.push(new_task.clone());
                        let raw = self.tasks.clone();
                        self.tasks = TodoTask::organize_hierarchy(raw);
                        self.input_value.clear();
                        if let Some(client) = &self.client {
                            return Task::perform(
                                async_create_wrapper(client.clone(), new_task),
                                Message::SyncSaved,
                            );
                        }
                    }
                }
                Task::none()
            }

            Message::ToggleTask(index, _checked) => {
                if let Some(task) = self.tasks.get_mut(index) {
                    // Optimistic flip
                    task.completed = !task.completed;
                    // Prepare clone for server (pre-flip state because toggle_task flips it)
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

    fn view(&self) -> Element<'_, Message> {
        let sidebar_content = column(
            self.calendars
                .iter()
                .map(|cal| {
                    let is_active = self.active_cal_href.as_ref() == Some(&cal.href);
                    let btn = button(text(&cal.name).size(16))
                        .padding(10)
                        .width(Length::Fill)
                        .on_press(Message::SelectCalendar(cal.href.clone()));
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
            .width(200)
            .height(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style::default()
                    .background(Background::Color(palette.background.weak.color))
            });

        let title_text = if self.loading {
            "Loading..."
        } else {
            "Rustache"
        };
        let search_input = text_input("Search...", &self.search_value)
            .on_input(Message::SearchChanged)
            .padding(5)
            .size(16);

        let input_placeholder = if self.editing_uid.is_some() {
            "Edit Title..."
        } else {
            "Add task (Buy cat food !1 @daily)..."
        };
        let input_title = text_input(input_placeholder, &self.input_value)
            .on_input(Message::InputChanged)
            .on_submit(Message::SubmitTask)
            .padding(10)
            .size(20);

        let footer_content: Element<_> = if self.editing_uid.is_some() {
            let input_desc = text_input("Notes...", &self.description_value)
                .on_input(Message::DescriptionChanged)
                .on_submit(Message::SubmitTask)
                .padding(10)
                .size(16);
            let cancel_btn = button(text("Cancel").size(16))
                .style(button::secondary)
                .on_press(Message::CancelEdit);
            let save_btn = button(text("Save").size(16))
                .style(button::primary)
                .on_press(Message::SubmitTask);
            column![
                row![
                    text("Editing")
                        .size(14)
                        .color(Color::from_rgb(0.7, 0.7, 1.0)),
                    horizontal_space(),
                    cancel_btn,
                    save_btn
                ]
                .spacing(10),
                input_title,
                input_desc
            ]
            .spacing(5)
            .into()
        } else {
            column![input_title].into()
        };

        let is_searching = !self.search_value.is_empty();
        let filtered_tasks: Vec<(usize, &TodoTask)> = self
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                if is_searching {
                    t.summary
                        .to_lowercase()
                        .contains(&self.search_value.to_lowercase())
                } else {
                    true
                }
            })
            .collect();

        let tasks_view: Element<_> = column(
            filtered_tasks
                .into_iter()
                .map(|(real_index, task)| {
                    let color = match task.priority {
                        1..=4 => Color::from_rgb(0.8, 0.2, 0.2),
                        5 => Color::from_rgb(0.8, 0.8, 0.2),
                        _ => Color::WHITE,
                    };

                    let indent_size = if is_searching { 0 } else { task.depth * 20 };
                    let indent = horizontal_space().width(Length::Fixed(indent_size as f32));

                    let summary = text(&task.summary)
                        .size(20)
                        .color(color)
                        .width(Length::Fill);

                    let date_text = match task.due {
                        Some(d) => text(d.format("%Y-%m-%d").to_string())
                            .size(14)
                            .color(Color::from_rgb(0.5, 0.5, 0.5)),
                        None => text(""),
                    };
                    let date_container = container(date_text)
                        .width(Length::Fixed(90.0))
                        .align_x(iced::alignment::Horizontal::Right);

                    let recur_text = if task.rrule.is_some() {
                        text("(R)").size(14).color(Color::from_rgb(0.6, 0.6, 1.0))
                    } else {
                        text("")
                    };
                    let recur_container = container(recur_text)
                        .width(Length::Fixed(30.0))
                        .align_x(iced::alignment::Horizontal::Center);

                    let btn_style = button::secondary;
                    let has_desc = !task.description.is_empty();
                    let is_expanded = self.expanded_tasks.contains(&task.uid);

                    // Buttons center content by default, so setting width on the button is enough.
                    let info_btn = if has_desc {
                        button(text("i").size(12))
                            .style(if is_expanded {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .padding(5)
                            .width(Length::Fixed(25.0))
                            .on_press(Message::ToggleDetails(task.uid.clone()))
                    } else {
                        button(text("").size(12))
                            .style(button::text)
                            .padding(5)
                            .width(Length::Fixed(25.0))
                    };

                    let actions = row![
                        info_btn,
                        button(text("+").size(14))
                            .style(btn_style)
                            .padding(5)
                            .on_press(Message::ChangePriority(real_index, 1)),
                        button(text("-").size(14))
                            .style(btn_style)
                            .padding(5)
                            .on_press(Message::ChangePriority(real_index, -1)),
                        button(text(">").size(14))
                            .style(btn_style)
                            .padding(5)
                            .on_press(Message::IndentTask(real_index)),
                        button(text("<").size(14))
                            .style(btn_style)
                            .padding(5)
                            .on_press(Message::OutdentTask(real_index)),
                        button(text("Edit").size(14))
                            .style(btn_style)
                            .padding(5)
                            .on_press(Message::EditTaskStart(real_index)),
                        button(text("Del").size(14))
                            .style(button::danger)
                            .padding(5)
                            .on_press(Message::DeleteTask(real_index)),
                    ]
                    .spacing(5);

                    let row_main = row![
                        indent,
                        checkbox("", task.completed)
                            .on_toggle(move |b| Message::ToggleTask(real_index, b)),
                        summary,
                        date_container,
                        recur_container,
                        actions
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center);

                    let content: Element<_> = if is_expanded {
                        let desc_text = text(&task.description)
                            .size(14)
                            .color(Color::from_rgb(0.7, 0.7, 0.7));
                        let desc_row = row![
                            horizontal_space().width(Length::Fixed(indent_size as f32 + 30.0)),
                            desc_text
                        ];
                        column![row_main, desc_row].spacing(5).into()
                    } else {
                        row_main.into()
                    };

                    container(content)
                        .padding(Padding {
                            top: 5.0,
                            right: 30.0,
                            bottom: 5.0,
                            left: 5.0,
                        })
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(2)
        .into();

        let main_content = column![
            row![
                text(title_text).size(40),
                horizontal_space(),
                search_input.width(200)
            ]
            .align_y(iced::Alignment::Center),
            footer_content,
            scrollable(tasks_view)
        ]
        .spacing(20)
        .padding(20)
        .max_width(800);
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

// --- ASYNC WRAPPERS & LOGIC (Same as before) ---
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
        let tasks = client.get_tasks().await.map_err(|e| e.to_string())?;
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
    let calendars = client.get_calendars().await.unwrap_or_default();
    let mut active_href = None;
    if let Some(def_cal) = config.default_calendar {
        if let Some(found) = calendars
            .iter()
            .find(|c| c.name == def_cal || c.href == def_cal)
        {
            client.set_calendar(&found.href);
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
    let tasks = client.get_tasks().await.map_err(|e| e.to_string())?;
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
