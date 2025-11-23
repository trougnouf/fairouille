use crate::client::RustyClient;
use crate::config::Config;
use crate::model::{CalendarListEntry, Task as TodoTask};

#[derive(Debug, Clone)]
pub enum Message {
    ObUrlChanged(String),
    ObUserChanged(String),
    ObPassChanged(String),
    ObDefaultCalChanged(String),
    ObSubmit,
    OpenSettings,
    CancelSettings,
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
    ConfigLoaded(Result<Config, String>),
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
